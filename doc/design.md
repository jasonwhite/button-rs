# Design

This document describes the high-level design of Button.

## Goals

 * Fast builds
 * Correct incremental builds
 * Implicit dependency detection(?)
 * Flexibility in the build description language. Just generate the JSON build
   description from any language of one's choosing.

## Implicit Dependency Detection

### Pros

 1. No need to specify header file dependencies or do (potentially expensive)
    pre-processing to determine all dependencies. This eases maintenance.
 2. Allows automatic deletion of unspecified build outputs.
 3. Allows recursive builds. A build system can track the inputs/outputs of a
    child build system and rebuild as necessary. This has the disadvantage of
    duplicating all the input/output nodes of a child build into the parent
    build. Often, all the parent build does is generate the build description
    and run a build on it.

### Cons

 1. This prevents caching of build artifacts at the build system level. It does
    not appear possible to determine if a task needs to be executed if we don't
    know all of the dependencies up front. This can only be done if tasks are
    made more course-grained. To avoid specifying all dependencies up front,
    course-grained tasks need to be baked into the build system. This is less
    flexible and also prevents automatic conversion from other build systems.
 2. Prevents hermetic builds. It's not possible to do sandboxing when not all
    dependencies are known up-front.
 3. Prevents distributed builds if not all dependencies are known up-front.
 4. Complicates the implementation.
    - Need to implement the detection via ad-hoc methods or via slow syscall
      tracing.
    - Need to allow recursive builds.
    - Many edge cases to consider.
      * What do we do if a cyclic dependency is created?
      * What do we do if a race condition is introduced?
