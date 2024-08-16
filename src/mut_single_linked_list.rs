#![allow(dead_code, unused)]

use std::{marker::PhantomData, mem};

/// a workable declaration of linked list from the functional programming perspective (copied from Scala) that 
/// requires only the addition of Box in the non-empty list variant to make sure the type is sized to compile in Rust
/// However, such declaration would lead to inefficient memory layout in further implementation down the line ?!
pub enum LinkedListBadLayout {
    // the end of the list, or a empty item
    Nil,
    // a non-empty item, with a pointer to the next item in the list
    // Noting the use of a fixed i32 type for data item, just for illustration purpose
    // as a starting point of the example
    Cons(i32, Box<LinkedListBadLayout>)
}

/// Such design of the entities of in a linked list would lead to a efficient memory layout ?!
pub struct LinkedList<T> {
    /// this type models a handle to the linked list, as convention, by accessing it from the head
    // head: LinkInvented<T>,
    head: Link<T>
}

// enum LinkInvented<T> {
//     /// this type models a pointer to the next item in the linked list that is aware of the two possible types of the item
//     /// i.e. Nil or a non-empty item
//     /// with the scrutiny of the declaration which expresses the intent of the structure, we see that it ensembles that of
//     /// the std::Option type exactly s.t. it is unecessary to reinvent the wheel
//     Empty,
//     NonEmpty(Box<Node<T>>),
// }

type Link<T> = Option<Box<Node<T>>>;
// enum Link {
//     None,
//     Some(Box<Node>),
// }

struct Node<T> {
    /// this types is essentially equivalent to what Cons is in Scala, i.e. and non-empty item in the linked list
    /// that has two fields, the data of the item itself, and a pointer to the next item
    data: T,
    next: Link<T>,
}

/// impl LinkedList with public APIs resembling those provided by a stack
/// 
impl<T> LinkedList<T> {

    pub fn peek(&self) -> Option<&T> {
        match self.head {
            None => {
                return None;
            },
            Some(ref ref_to_boxed_node) => {
                // the mechanics of automatic dereference of Rust enables that, having
                // a &Box<Node<T>> at hand implies having &Node<T>, which gives the &T
                // required for the implementation of the API
                return Some(&ref_to_boxed_node.data);
            }
        }
    }

    /// implementation that would create a new boxed node with the given input data and the pointer re-using the
    /// existing link of the current head, and update the current head be the link to this new boxed node
    pub fn append(&mut self, value: T) {
        /// implementation trick is to obtain the link of the current head by replacing it out with a temporary None Link
        /// to complete the creation of the new boxed node
        let new_boxed_node = Box::new(
            Node {
                data: value,
                next: mem::replace(&mut self.head, None)
            }
        );

        self.head = Some(new_boxed_node)
    }
    
    pub fn pop_front(&mut self) -> Option<T> {
        /// implementation trick, similar to that in append API, is to replacing the Link
        /// out of the head field gain ownership into the boxed node by binding it with a 
        /// local variable s.t. the node's data and link to the (originally) second next
        /// can be accessed
        match mem::replace(&mut self.head, None) {
            None => {
                return None;
            },
            Some(boxed_node_to_pop) => {
                
                // do what needs to be done for the API implementation with the boxed node
                self.head = boxed_node_to_pop.next;
                return Some(boxed_node_to_pop.data);

            // the binding of the boxed node to the local var conviently
            // implies that after we're done with the business of accessing
            // stuff out of the boxed node, the box is also correctly deallocated
            // by the virtue of its owning variable going out of scope
            }
        }
    }

    /// impl public-facing APIs that adapt LinkList into Iterators
    /// following the convention of such utitily APIs provided by common collections in std
    /// three flavors of Iterators is usually provided to callers
    /// - an Iterator yielding owned T, provided an owned instanced of LinkedList<T>
    /// - an Iterator yielding &T, provided a &LinkedList<T>
    /// - an Iteraotr yield &mut T, provided a &mut LinkList<T>
    /// By providing the implementation of the IntoIterator trait for LinkedList<T>, along with its required backing type
    /// that implements Iterator<Item=T>, the first sort of the tool expected by the callers is delivered by
    /// using the into_iter public interface
    /// Hence what's left is to provide the other two public interfaces and give their backing implementations
    pub fn iter(&self) -> LinkedListIter<T> {
        LinkedListIter {
            next_item: &self.head,
        }
    }

    // pub fn iter_mut<'a, 'b>(&'a mut self) -> LinkedListIterMut<'b, T> 
    // where 'a: 'b
    // {
    //     LinkedListIterMut {
    //         next_item: &mut self.head
    //     }
    // }

    pub fn iter_mut<'a>(&'a mut self) -> LinkedListIterMut<'a, T> 
    {   
        LinkedListIterMut {
            // next_item: &mut self.head,
            next_item: self.head.as_mut() // Option<&mut Box<Node<T>>>
            // next_item: self.head.as_deref_mut() // Option<&mut Node<T>>
        }
    }

}

/// backing impl for providing Iterator<Item = &'a T>, given &'a LinkList<T>
pub struct LinkedListIter<'a, T> {
    // provided &'a LinkedList<T>, it is ok to have &'a Link<T> extracted from
    // the head field of that, and sticks it in the LinkedListIter<'a, T>
    next_item: &'a Link<T>
}

impl<'a, T> Iterator for LinkedListIter<'a, T> {
    type Item = &'a T;

    fn next<'b>(self: &'b mut LinkedListIter<'a, T>) -> Option<&'a T> {
        /// the implementation of the API needs to fulfill two requirements, which is to
        /// first and foremost, return a reference to the data item, &T, to the caller,
        /// and on the otherhand, update the instance to have the reference to the "next"
        /// link, to be ready for the subsequent calls
        match *self.next_item {
            None => {
                // in case of being pointing to the end of the LinkedList
                return None;
            },
            Some(ref ref_to_boxed_node) => {
                let next_link: &'a Link<T> = &self.next_item.as_ref().unwrap().next;
                self.next_item = next_link;
                return Some(&ref_to_boxed_node.data);
            }
        }
    }
}

/// backing impl for providing Iterator<Item = &'a mut T>, given &'a mut LinkList<T>
/// probably a naive solution by blindly following the implementation given for Iter 
/// that if implemented as is, would imply that with the given
/// &'a mut LinkList<T> and call the next method provided on the "would be" provided
/// Iterator<Item = 'a mut T>, the caller "would have" obtained potentially more than one
/// &'a mut T, which would be in violation of the borrowing rule of safe Rust 
// pub struct LinkedListIterMut<'a, T> {
//     next_item: &'a mut Link<T>,
// }

// impl<'a, T> Iterator for LinkedListIterMut<'a, T> {
//     type Item = &'a mut T;

//     fn next(&mut self) -> Option<Self::Item> {
//         match *self.next_item {
//             None => {
//                 return None;
//             },
//             Some(ref mut ref_to_boxed_node) => {
//                 self.next_item = &mut ref_to_boxed_node.next;
//                 return Some(&mut ref_to_boxed_node.data);
//             }
//         }
//     }
// }

pub struct LinkedListIterMut<'a, T> {
    // next_item: &'a mut Option<Box<Node<T>>>,
    next_item: Option<&'a mut Box<Node<T>>>,
    // next_item: Option<&'a mut Node<T>>,
}

impl <'a, T> Iterator for LinkedListIterMut<'a, T> {
    type Item = &'a mut T;

    fn next<'c>(self: &'c mut LinkedListIterMut<'a, T>) -> Option<&'a mut T>
    {
        // match self.next_item {
        //     None => {},
        //     Some(ref mut v) => {
        //         // self.next_item = &mut v.next;

        //         // self.next.take().map(|node| {
        //         //     self.next = node.next.as_deref_mut();
        //         //     &mut node.elem
        //         // })
        //     }
        // }
        if self.next_item.is_none() {

        } else {
            // return Some(&mut self.next_item.as_mut().unwrap().data);
            
            let _t: Option<&'a mut Box<Node<T>>> = self.next_item.take();
            // return Some(&mut self.next_item.take().unwrap().data);
        }

        None
    }
}



/// backing impl for providing Iterator<Item = T>, given owned LinkList<T>
pub struct LinkedListIntoIter<T> {
    inner: LinkedList<T>,
}

impl<T> Iterator for LinkedListIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop_front()
    }
}

impl<T> IntoIterator for LinkedList<T> {
    type Item = T;

    type IntoIter = LinkedListIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        LinkedListIntoIter {
            inner: self,
        }
    }
}


