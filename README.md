# Tune your SmallVecs!

*This is a Work In Progress. Issues and Pull Requests Welcome!*

You can use this crate to get information on how your code uses `SmallVec`s.
What it does is writing a log of all SmallVec constructions, resizings and
destructions by array size.

Each line in the log is composed of `<id>;<item size>;<array
size>;[+/-];<previous capacity>;<new capacity>`, where the first item is the
size of the array's item type, the second is the array size (we can use this to
distinguish various smallvec uses within one application), the third is `+` for
a new allocation, `-` for a deallocation and space on resize, and the fourth
and fifth are the capacity before and after the operation (if any).

For example, creating a smallvec of `u8`, extending and dropping it may create
the following log (here annotated for clarity):

```
0;1;1;+;;1      # create
0;1;1; ;1;100   # extend (allocate+deallocate)
0;1;1;-;100     # drop
```

# License

# Usage

In your `Cargo.toml`, replace your `smallvec` dependency with `smallvectune`. Then
in your lib.rs (or main.rs), replace `extern crate smallvec;` with
`extern crate smallvectune as smallvec;`.

Calling your code, you'll have to set the `SMALLVECTUNE_OUT` environment variable
to a valid path to write to. This is where the log will be written.

# License

This is under Apache/2 or MIT license, per your choice. All contributions
are also given under the same license.
