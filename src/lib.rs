//! Tuning utility for smallvec
//!
//! You can use this crate as a stand-in for smallvec to get a number of
//! files you can use to decide on good smallvec sizes.
//!
//! Use with `extern crate smallvectune; use smallvectune::SmallVec` instead
//! of `extern crate smallvec; use smallvec::SmallVec`.

extern crate smallvec;
extern crate crossbeam_channel;
#[macro_use]
extern crate lazy_static;

use std::{fmt, fs, mem, ops, thread};
use std::borrow::{Borrow, BorrowMut};
use std::io::{self, Write, BufWriter};
use std::iter::FromIterator;
use std::path::Path;
use std::sync::Mutex;

pub use smallvec::{Array, Drain, ExtendFromSlice, IntoIter};
use smallvec::SmallVec as SV;

// public API
#[macro_export]
macro_rules! smallvec {
    // count helper: transform any expression into 1
    (@one $x:expr) => (1usize);
    ($elem:expr; $n:expr) => ({
        $crate::SmallVec::from_elem($elem, $n)
    });
    ($($x:expr),*$(,)*) => ({
        let count = 0usize $(+ smallvec!(@one $x))*;
        let mut vec = $crate::SmallVec::new();
        if count <= vec.inline_size() {
            $(vec.push($x);)*
            vec
        } else {
            $crate::SmallVec::from_vec(vec![$($x,)*])
        }
    });
}

#[cfg(not(feature = "id"))]
mod id {
    pub type Id = ();
    pub fn next_id() -> Id { }
    pub fn write_id(w: &mut BufWriter<fs::File>, id: Id) { }
}

#[cfg(feature = "id")]
mod id {
    use std::fs;
    use std::io::{Write, BufWriter};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ID: AtomicUsize = AtomicUsize::new(0);

    pub type Id = usize;
    pub fn next_id() -> Id { ID.fetch_add(1, Ordering::SeqCst) }
    pub fn write_id(w: &mut BufWriter<fs::File>, id: Id) { write!(w, "{};", id); }
}

use id::*;

/// Our wrapped SmalLVec type
pub struct SmallVec<A: Array>(SV<A>, Id);

macro_rules! delegate {
    { $name:ident ( $($arg:ident : $ty:ty),* ) } => {
        delegate! { $name ( $($arg : $ty),* ) -> () }
    };
    { $name:ident ($($arg:ident : $ty:ty),*) -> $ret:ty } => {
        #[inline]
        pub fn $name(&self, $($arg: $ty,)*) -> $ret {
            self . 0 . $name($($arg,)*)
        }
    };
}

macro_rules! delegate_mut {
    { $name:ident ( $($arg:ident : $ty:ty),* ) } => {
        delegate_mut! { $name ( $($arg : $ty),* ) -> () }
    };
    { $name:ident ($($arg:ident : $ty:ty),*) -> $ret:ty } => {
        #[inline]
        pub fn $name(&mut self, $($arg: $ty,)*) -> $ret {
            let previous_cap = self.0.capacity();
            let result = self . 0 . $name($($arg,)*);
            let new_cap = self.0.capacity();
            resize(self.1, mem::size_of::<A::Item>(), A::size(), previous_cap, new_cap);
            result
        }
    };
}

impl<A: Array> SmallVec<A> {
    #[inline]
    pub fn new() -> SmallVec<A> {
        SmallVec(SV::new(), hi(mem::size_of::<A::Item>(), A::size(), 0))
    }

    #[inline]
    pub fn with_capacity(cap: usize) -> SmallVec<A> {
        SmallVec(SV::with_capacity(cap), hi(mem::size_of::<A::Item>(), A::size(), cap))
    }

    #[inline]
    pub fn from_vec(vec: Vec<A::Item>) -> SmallVec<A> {
        let sv = SV::from_vec(vec);
        let capacity = sv.capacity();
        SmallVec(sv, hi(mem::size_of::<A::Item>(), A::size(), capacity))
    }

    #[inline]
    pub fn from_buf(buf: A) -> SmallVec<A> {
        SmallVec(SV::from_buf(buf), hi(mem::size_of::<A::Item>(), A::size(), A::size()))
    }

    #[inline]
    pub fn from_buf_and_len(buf: A, len: usize) -> SmallVec<A> {
        SmallVec(SV::from_buf_and_len(buf, len),
                 hi(mem::size_of::<A::Item>(), A::size(), A::size()))
    }

    #[inline]
    pub unsafe fn from_buf_and_len_unchecked(buf: A, len: usize) -> SmallVec<A> {
        SmallVec(SV::from_buf_and_len_unchecked(buf, len),
                 hi(mem::size_of::<A::Item>(), A::size(), A::size()))
    }

    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.0.set_len(new_len)
    }

    #[inline]
    pub fn inline_size(&self) -> usize {
        A::size()
    }

    delegate! { len() -> usize }
    delegate! { is_empty() -> bool }
    delegate! { capacity() -> usize }
    delegate! { spilled() -> bool }

    #[inline]
    pub fn drain(&mut self) -> Drain<A::Item> {
        bye(self.1, mem::size_of::<A::Item>(), A::size(), self.0.capacity());
        self.0.drain()
    }

    delegate_mut! { push(value: A::Item) }

    #[inline]
    pub fn pop(&mut self) -> Option<A::Item> {
        self.0.pop()
    }

    delegate_mut! { grow(new_cap: usize) }
    delegate_mut! { reserve(additional: usize) }
    delegate_mut! { reserve_exact(additional: usize) }
    delegate_mut! { shrink_to_fit() }

    #[inline]
    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len)
    }

    delegate! { as_slice() -> &[A::Item] }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [A::Item] {
        self.0.as_mut_slice()
    }

    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> A::Item {
        self.0.swap_remove(index)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[inline]
    pub fn remove(&mut self, index: usize) -> A::Item {
        self.0.remove(index)
    }

    delegate_mut! { insert(index: usize, element: A::Item) }

    #[inline]
    pub fn insert_many<I: IntoIterator<Item=A::Item>>(&mut self, index: usize, iterable: I) {
        let previous_cap = self.0.capacity();
        let result = self.0.insert_many(index, iterable);
        let new_cap = self.0.capacity();
        resize(self.1, mem::size_of::<A::Item>(), A::size(), previous_cap, new_cap);
        result
    }

    #[inline]
    pub fn into_vec(mut self) -> Vec<A::Item> {
        bye(self.1, mem::size_of::<A::Item>(), A::size(), self.0.capacity());
        let sv = mem::replace(&mut self.0, SV::new());
        mem::forget(self);
        sv.into_vec()
    }

    #[inline]
    pub fn into_inner(mut self) -> Result<A, Self> {
        let (sv, id) = (mem::replace(&mut self.0, SV::new()), self.1);
        mem::forget(self);
        match sv.into_inner() {
            Ok(a) => {
                bye(id, mem::size_of::<A::Item>(), A::size(), A::size());
                Ok(a)
            }
            Err(s) => Err(SmallVec(s, id))
        }
    }

    #[inline]
    pub fn retain<F: FnMut(&mut A::Item) -> bool>(&mut self, f: F) {
        self.0.retain(f);
    }

    #[inline]
    pub fn dedup(&mut self) where A::Item: PartialEq<A::Item> {
        self.0.dedup();
    }

    #[inline]
    pub fn dedup_by<F>(&mut self, same_bucket: F)
        where F: FnMut(&mut A::Item, &mut A::Item) -> bool
    {
        self.0.dedup_by(same_bucket)
    }

    #[inline]
    pub fn dedup_by_key<F, K>(&mut self, key: F)
        where F: FnMut(&mut A::Item) -> K,
              K: PartialEq<K>
    {
        self.0.dedup_by_key(key)
    }
}

impl<A: Array> SmallVec<A> where A::Item: Copy {
    #[inline]
    pub fn from_slice(slice: &[A::Item]) -> Self {
        let sv = SV::from_slice(slice);
        let capacity = sv.capacity();
        SmallVec(sv, hi(mem::size_of::<A::Item>(), A::size(), capacity))
    }

    delegate_mut! { insert_from_slice(index: usize, slice: &[A::Item]) }
    delegate_mut! { extend_from_slice(slice: &[A::Item]) }
}

impl<A: Array> SmallVec<A> where A::Item: Clone {
    delegate_mut! { resize(len: usize, value: A::Item) }

    #[inline]
    pub fn from_elem(elem: A::Item, n: usize) -> Self {
        let sv = SV::from_elem(elem, n);
        let capacity = sv.capacity();
        SmallVec(sv, hi(mem::size_of::<A::Item>(), A::size(), capacity))
    }
}

impl<A: Array> ops::Deref for SmallVec<A> {
    type Target = [A::Item];
    #[inline]
    fn deref(&self) -> &[A::Item] {
        self.0.deref()
    }
}

impl<A: Array> ops::DerefMut for SmallVec<A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [A::Item] {
        self.0.deref_mut()
    }
}

impl<A: Array> AsRef<[A::Item]> for SmallVec<A> {
    #[inline]
    fn as_ref(&self) -> &[A::Item] {
        self
    }
}

impl<A: Array> AsMut<[A::Item]> for SmallVec<A> {
    #[inline]
    fn as_mut(&mut self) -> &mut [A::Item] {
        self
    }
}

impl<A: Array> Borrow<[A::Item]> for SmallVec<A> {
    #[inline]
    fn borrow(&self) -> &[A::Item] {
        self
    }
}

impl<A: Array> BorrowMut<[A::Item]> for SmallVec<A> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [A::Item] {
        self
    }
}

#[cfg(feature = "std")]
impl<A: Array<Item = u8>> io::Write for SmallVec<A> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let previous_cap = self.0.capacity();
        let result = self.0.write(buf);
        let new_cap = self.0.capacity();
        resize(self.1, mem::size_of::<A::Item>(), A::size(), previous_cap, new_cap);
        result
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let previous_cap = self.0.capacity();
        let result = self.0.write_all(buf);
        let new_cap = self.0.capacity();
        resize(self.1, mem::size_of::<A::Item>(), A::size(), previous_cap, new_cap);
        result
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "serde")]
impl<A: Array> Serialize for SmallVec<A> where A::Item: Serialize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, A: Array> Deserialize<'de> for SmallVec<A> where A::Item: Deserialize<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let sv = SV::deserialize(deserializer);
        let capacity = sv.capacity();
        SmallVec(sv, hi(mem::size_of::<A::Item>(), A::size(), capacity))
    }
}

impl<A: Array, T> From<T> for SmallVec<A> where T: Into<SV<A>> {
    #[inline]
    fn from(t: T) -> SmallVec<A> {
        let sv = t.into();
        let capacity = sv.capacity();
        SmallVec(sv, hi(mem::size_of::<A::Item>(), A::size(), capacity))
    }
}

macro_rules! impl_index {
    ($index_type: ty, $output_type: ty) => {
        impl<A: Array> ops::Index<$index_type> for SmallVec<A> {
            type Output = $output_type;
            #[inline]
            fn index(&self, index: $index_type) -> &$output_type {
                &(&**self)[index]
            }
        }

        impl<A: Array> ops::IndexMut<$index_type> for SmallVec<A> {
            #[inline]
            fn index_mut(&mut self, index: $index_type) -> &mut $output_type {
                &mut (&mut **self)[index]
            }
        }
    }
}

impl_index!(usize, A::Item);
impl_index!(ops::Range<usize>, [A::Item]);
impl_index!(ops::RangeFrom<usize>, [A::Item]);
impl_index!(ops::RangeTo<usize>, [A::Item]);
impl_index!(ops::RangeFull, [A::Item]);

impl<A: Array> ExtendFromSlice<A::Item> for SmallVec<A> where A::Item: Copy {
    #[inline]
    fn extend_from_slice(&mut self, slice: &[A::Item]) {
        let previous_cap = self.0.capacity();
        self.0.extend_from_slice(slice);
        let new_cap = self.0.capacity();
        resize(self.1, mem::size_of::<A::Item>(), A::size(), previous_cap, new_cap);
    }
}

impl<A: Array> FromIterator<A::Item> for SmallVec<A> {
    #[inline]
    fn from_iter<I: IntoIterator<Item=A::Item>>(iterable: I) -> SmallVec<A> {
        let sv = SV::from_iter(iterable);
        let capacity = sv.capacity();
        SmallVec(sv, hi(mem::size_of::<A::Item>(), A::size(), capacity))
    }
}

impl<A: Array> Extend<A::Item> for SmallVec<A> {
    fn extend<I: IntoIterator<Item=A::Item>>(&mut self, iterable: I) {
        let previous_cap = self.0.capacity();
        self.0.extend(iterable);
        let new_cap = self.0.capacity();
        resize(self.1, mem::size_of::<A::Item>(), A::size(), previous_cap, new_cap);
    }
}

impl<A: Array> fmt::Debug for SmallVec<A> where A::Item: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<A: Array> Default for SmallVec<A> {
    #[inline]
    fn default() -> SmallVec<A> {
        SmallVec::new()
    }
}

impl<A: Array> Drop for SmallVec<A> {
    fn drop(&mut self) {
        bye(self.1, mem::size_of::<A::Item>(), A::size(), self.capacity());
    }
}

impl<A: Array> Clone for SmallVec<A> where A::Item: Clone {
    fn clone(&self) -> SmallVec<A> {
        SmallVec(self.0.clone(),
                 hi(mem::size_of::<A::Item>(), A::size(), self.capacity()))
    }
}

struct ArrayInfo {
    id: Id,
    item_size: usize,
    array_size: usize,
}

enum Message {
    New(ArrayInfo, usize),
    Resize(ArrayInfo, usize, usize),
    Drop(ArrayInfo, usize),
    Quit
}

pub struct Logger(crossbeam_channel::Sender<Message>,
                  Mutex<Option<thread::JoinHandle<()>>>);

lazy_static! {
    static ref LOG: Logger = {
        let out = std::env::var("SMALLVECTUNE_OUT")
            .expect("Please set SMALLVECTUNE_OUT=path/to/out.csv");
        let path: &Path = Path::new(&out);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .expect("Could not create output directory");
        }
        let file = fs::File::create(path)
            .expect("Could not create output file");
        let buf_writer = BufWriter::new(file);
        let (s, r) = crossbeam_channel::bounded(128); // should be enough
        let join = thread::spawn(move || {
            let mut buf = buf_writer;
            loop {
                let buf = &mut buf;
                match r.recv() {
                    Some(Message::New(info, cap)) =>
                        line(buf, info, '+', None, Some(cap)),
                    Some(Message::Drop(info, cap)) =>
                        line(buf, info, '-', Some(cap), None),
                    Some(Message::Resize(info, previous_cap, new_cap)) => {
                        line(buf, info, ' ', Some(previous_cap), Some(new_cap))
                    }
                    Some(Message::Quit) | None => {
                        let _ = buf.flush();
                        break
                    }
                }
            }
            drop(buf);
        });
        Logger(s, Mutex::new(Some(join)))
    };
}

pub struct Log;

impl Drop for Log {
    fn drop(&mut self) {
        LOG.0.send(Message::Quit);
        let mut lock = LOG.1.lock().unwrap();
        if let Some(j) = lock.take() {
            let _ = j.join();
        }
    }
}

/// Use this with `let _log = with_log();` in your main method to flush the log
/// and exit the logger thread on program exit
pub fn with_log() -> Log { Log }

fn write_opt(w: &mut BufWriter<fs::File>, o: Option<usize>, c: &str) {
    match o {
        Some(value) => { let _ = write!(w, "{}{}", value, c); }
        None => { let _ = w.write(c.as_bytes()); }
    }
}

fn line(w: &mut BufWriter<fs::File>,
        info: ArrayInfo,
        addrem: char,
        previous_cap: Option<usize>,
        new_cap: Option<usize>) {
    let ArrayInfo { id, item_size, array_size } = info;
    write_id(w, id);
    write!(w, "{};{};{};", item_size, array_size, addrem);
    write_opt(w, previous_cap, ";");
    write_opt(w, new_cap, "\n");
    let _ = w.flush(); // nothing to do about errors here
}

fn send(message: Message) {
    LOG.0.send(message);
}

fn hi(item_size: usize, array_size: usize, capacity: usize) -> Id {
    let id = next_id();
    send(Message::New(ArrayInfo { id, item_size, array_size }, capacity));
    id
}

fn resize(id: Id,
          item_size: usize,
          array_size: usize,
          previous_capacity: usize,
          new_capacity: usize) {
    if new_capacity != previous_capacity {
        send(Message::Resize(ArrayInfo { id, item_size, array_size },
                             previous_capacity,
                             new_capacity));
    }
}

fn bye(id: Id, item_size: usize, array_size: usize, capacity: usize) {
    send(Message::Drop(ArrayInfo { id, item_size, array_size }, capacity));
}
