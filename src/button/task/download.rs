// Copyright (c) 2017 Jason White
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time;

use reqwest;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::detect::Detected;
use crate::error::{Error, ResultExt};
use crate::res;
use crate::util::{progress_dummy, Retry, Sha256, ShaVerifyError};

use super::traits::Task;

/// A task to download a URL. This would normally be a task with no input
/// resources.
#[derive(
    Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone,
)]
pub struct Download {
    /// Process and arguments to spawn.
    url: String,

    /// Expected SHA256 hash.
    sha256: Sha256,

    /// Output path.
    path: PathBuf,

    /// How much time to give it to download. If `None`, there is no time
    /// limit.
    timeout: Option<time::Duration>,

    /// Retry settings.
    #[serde(default)]
    retry: Retry,
}

impl fmt::Display for Download {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.url)
    }
}

impl fmt::Debug for Download {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.url)
    }
}

impl Task for Download {
    fn execute(
        &self,
        root: &Path,
        log: &mut dyn io::Write,
    ) -> Result<Detected, Error> {
        let path = root.join(&self.path);

        writeln!(log, "Downloading \"{}\" to {:?}", self.url, self.path)?;

        // Retry the download if necessary.
        let mut response = self.retry.call(
            || reqwest::get(&self.url)?.error_for_status(),
            progress_dummy,
        )?;

        // Download to a temporary file that sits next to the desired path.
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mut temp = NamedTempFile::new_in(dir)?;

        response.copy_to(&mut io::BufWriter::new(&mut temp))?;

        let temp = temp.into_temp_path();

        // Verify the SHA256
        let sha256 = Sha256::from_path(&temp)?;
        if self.sha256 != sha256 {
            let result: Result<(), Error> =
                Err(ShaVerifyError::new(self.sha256.clone(), sha256).into());
            result.context(format!(
                "Failed to verify SHA256 for URL {}",
                self.url
            ))?;
        }

        temp.persist(&path)?;

        Ok(Detected::new())
    }

    fn known_outputs(&self, resources: &mut res::Set) {
        // TODO: Depend on output directory.
        resources.insert(self.path.clone().into());
    }
}
