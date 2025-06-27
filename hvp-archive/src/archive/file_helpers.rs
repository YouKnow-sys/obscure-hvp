//! this module make it easier to work with files inside archives

use std::{collections::VecDeque, path::PathBuf};

// TODO: maybe make the FileIterator more generic so we no longer
// need two seperate struct for imutable and mutable

use super::entry::{Entry, FullFileEntry, FullFileEntryMut};

struct StackFrame<E> {
    entry: E,
    depth: usize,
}

pub struct FileIterator<'a, 'p> {
    stack: VecDeque<StackFrame<&'a Entry<'p>>>,
    path_stack: Vec<&'a str>,
    files_count: usize,
    idx: usize,
}

impl<'a, 'p> FileIterator<'a, 'p> {
    pub(super) fn new(entries: &'a [Entry<'p>], files_count: usize) -> Self {
        let mut stack = VecDeque::with_capacity(entries.len());

        // Add entries in reverse order (so we process them in original order) at depth 0
        for entry in entries.iter().rev() {
            stack.push_back(StackFrame { entry, depth: 0 });
        }

        Self {
            stack,
            path_stack: Vec::new(),
            files_count,
            idx: 0,
        }
    }
}

impl<'a, 'p> Iterator for FileIterator<'a, 'p> {
    type Item = FullFileEntry<'p>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(frame) = self.stack.pop_back() {
            // match current depth
            self.path_stack.truncate(frame.depth);

            match frame.entry {
                Entry::File(file_entry) => {
                    // Build path from current path_stack + file name
                    let mut path: PathBuf = self.path_stack.iter().collect();
                    path.push(&file_entry.name);

                    let file = FullFileEntry {
                        path,
                        compression_info: file_entry.compression_info,
                        checksum: file_entry.checksum,
                        endian: file_entry.endian,
                        raw_bytes: file_entry.raw_bytes,
                    };

                    self.idx += 1;

                    return Some(file);
                }
                Entry::Dir(dir_entry) => {
                    self.path_stack.push(&dir_entry.name);

                    // Add children to stack with increased depth
                    let child_depth = frame.depth + 1;
                    for child_entry in dir_entry.entries.iter().rev() {
                        self.stack.push_back(StackFrame {
                            entry: child_entry,
                            depth: child_depth,
                        });
                    }
                }
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a, 'p> ExactSizeIterator for FileIterator<'a, 'p> {
    fn len(&self) -> usize {
        self.files_count - self.idx
    }
}

pub struct FileIteratorMut<'a, 'p> {
    stack: VecDeque<StackFrame<&'a mut Entry<'p>>>,
    path_stack: Vec<&'a str>,
    files_count: usize,
    idx: usize,
}

impl<'a, 'p> FileIteratorMut<'a, 'p> {
    pub(super) fn new(entries: &'a mut [Entry<'p>], files_count: usize) -> Self {
        let mut stack = VecDeque::with_capacity(entries.len());

        // Add entries in reverse order (so we process them in original order) at depth 0
        for entry in entries.iter_mut().rev() {
            stack.push_back(StackFrame { entry, depth: 0 });
        }

        Self {
            stack,
            path_stack: Vec::new(),
            files_count,
            idx: 0,
        }
    }
}

impl<'a, 'p> Iterator for FileIteratorMut<'a, 'p> {
    type Item = FullFileEntryMut<'a, 'p>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(frame) = self.stack.pop_back() {
            // match current depth
            self.path_stack.truncate(frame.depth);

            match frame.entry {
                Entry::File(file_entry) => {
                    // Build path from current path_stack + file name
                    let mut path: PathBuf = self.path_stack.iter().collect();
                    path.push(&file_entry.name);

                    let file = FullFileEntryMut {
                        path,
                        entry: file_entry,
                    };

                    self.idx += 1;

                    return Some(file);
                }
                Entry::Dir(dir_entry) => {
                    self.path_stack.push(&dir_entry.name);

                    // Add children to stack with increased depth
                    let child_depth = frame.depth + 1;
                    for child_entry in dir_entry.entries.iter_mut().rev() {
                        self.stack.push_back(StackFrame {
                            entry: child_entry,
                            depth: child_depth,
                        });
                    }
                }
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a, 'p> ExactSizeIterator for FileIteratorMut<'a, 'p> {
    fn len(&self) -> usize {
        self.files_count - self.idx
    }
}
