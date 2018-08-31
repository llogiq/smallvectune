# Tune your SmallVecs!

*This is a Work In Progress. Issues and Pull Requests Welcome!*

You can use this crate to get information on how your code uses `SmallVec`s.
What it does is writing a log of all SmallVec constructions, resizings and
destructions by array size.

Each line in the log is composed of `<array size>;[+/-];<capacity>`, where
the first item is the internal array size (we can use this to distinguish
various smallvec uses within one application), the second is `+` for a new
allocation and `-` for a deallocation and the third is the resulting
capacity.

For example, creating a smallvec, extending and dropping it may create the
following log (here annotated for clarity):

```
1;+;1       # create
1;+;100     # extend (allocate+deallocate)
1;-;1
1;-;100     # drop
```

# License

This is under Apache/2 or MIT license, per your choice. All contributions
are also given under the same license.
