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
use std::net::{Ipv4Addr, SocketAddr};

use button::{
    server::{Client, Request},
    Error,
};
use structopt::StructOpt;

use crate::opts::GlobalOpts;

#[derive(StructOpt, Debug)]
pub struct Test {
    /// Port of the server.
    #[structopt()]
    port: u16,
}

impl Test {
    pub fn main(self, _global: &GlobalOpts) -> Result<(), Error> {
        let socket =
            SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), self.port);

        let mut client = Client::connect(&socket)?;

        println!("Connected!");

        let (response, has_body) = client.request(Request::Build)?;
        println!("Got response: {:?}", response);
        if has_body {
            while let Some(item) = client.read_body_item()? {
                println!("Got body item: {:?}", item);
            }
        }

        Ok(())
    }
}
