#![allow(dead_code, unused)]

pub mod chennel_only_channel {
    use std::sync::Mutex;
    use std::sync::Condvar;
    use std::collections::VecDeque;

    pub struct Channel<T> {
        msg_queue: Mutex<VecDeque<T>>,
        recv_wakeup_flag: Condvar,
    }
    
    impl<T> Channel<T> {
        pub fn new() -> Self {
            Self {
                msg_queue: Mutex::new(VecDeque::default()),
                recv_wakeup_flag: Condvar::new(),
            }
        }
    
        pub fn send(&self, value: T) {
            let mut q_guard = self.msg_queue.lock().unwrap();
            q_guard.push_back(value);
            self.recv_wakeup_flag.notify_one();
        }
    
        pub fn recv(&self) -> T {
            let mut q_guard = self.msg_queue.lock().unwrap();
            loop {
                match q_guard.pop_front() {
                    None => {
                        q_guard = self.recv_wakeup_flag.wait(q_guard).unwrap();
                    },
                    Some(msg) => {
                        return msg;
                    }
                }
            }
        }
    }
}

mod tx_rx_channel {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::Condvar;
    use std::collections::VecDeque;
    
    pub struct Sender<T> {
        shared_inner: Arc<SharedInner<T>>,
    }

    impl<T> Sender<T> {
        pub fn send(&self, value: T) -> Result<(), NoMoreReceiverErr<T>> {
            // acquire lock to the mutable common data to access the msg queue to push a msg
            // dropping the lock guard to release the lock after the expression
            self.shared_inner.inner_mut_data.lock().unwrap().msg_queue.push_back(value);
            self.shared_inner.recv_wakeup_flag.notify_one();
            Ok(())
        }
    }

    pub struct NoMoreReceiverErr<T>(pub T);

    /// Clone and Drop, together, are all the interfaces on Sender that affect the count of senders
    /// in the mpsc setup, whose implementation is all it takes to keep track of the right count
    impl<T> Clone for Sender<T> {
        fn clone(&self) -> Self {
            self.shared_inner.inner_mut_data.lock().unwrap().sender_cnt += 1;
            // return a new Sender wrapping the shared inner with updated sender
            // count, as the return of the clone of an existing Sender
            Sender {
                shared_inner: Arc::clone(&self.shared_inner)
            }
        }
    }

    impl<T> Drop for Sender<T> {
        fn drop(&mut self) {
            let mut inner_mut_data_lock = self.shared_inner.inner_mut_data.lock().unwrap();
            inner_mut_data_lock.sender_cnt -= 1;
            dbg!(inner_mut_data_lock.sender_cnt);
            if inner_mut_data_lock.sender_cnt == 0 {
                drop(inner_mut_data_lock);
                self.shared_inner.recv_wakeup_flag.notify_one();
            }
        }
    }

    pub struct Receiver<T> {
        shared_inner: Arc<SharedInner<T>>,
    }

    #[derive(Debug)]
    pub struct NoMoreSenderErr;

    impl<T> Receiver<T> {
        
        /// bogus implementation of recv that would hang forever, in the case that there is no msg to receive from the 
        /// channel and all the senders have been dropped where there is no point to block waiting for msg any more
        pub fn tx_unaware_recv(&self) -> T {
            let mut shared_mut_data_guard = self.shared_inner.inner_mut_data.lock().unwrap();
            loop {
                if let Some(msg) = shared_mut_data_guard.msg_queue.pop_front() {
                    return msg;
                }
                // core implementation to enable the receive to become a blocking call in this case
                // when there is no current msg to receive from the channel is to meke use of the cond var that
                // is meant to notify the presence of a new sent msg s.t. the cond var is waited atomically with 
                // the release of lock held on this receiving end right now, and reacquiring the lock on the
                // cond var's notification to proceed into the next round of the loop, where the sent msg would be returned
                shared_mut_data_guard = self.shared_inner.recv_wakeup_flag.wait(shared_mut_data_guard).unwrap();
            }
        }

        pub fn recv(&self) -> Result<T, NoMoreSenderErr> {
            let mut shared_mut_data_guard = self.shared_inner.inner_mut_data.lock().unwrap();
            loop {
                if let Some(msg) = shared_mut_data_guard.msg_queue.pop_front() {
                    return Ok(msg);
                } else {
                    // here in the `else` branch due to the fact that the exucution of the call finds out that
                    // there is no data in the channel to receive, further divided in two cases
                    if shared_mut_data_guard.sender_cnt == 0 {
                        // in case that there is no sender in the mpsc setup, plus no msg to receive from the channel
                        return Err(NoMoreSenderErr)
                    } else {
                        // otherwise the receive becomes a blocking call that proceeds when further msg is sent by any sender
                        shared_mut_data_guard = self.shared_inner.recv_wakeup_flag.wait(shared_mut_data_guard).unwrap();
                    }
                }
            }
        }
    }
    
    // modelling the ONE common entity shared (by means of Arc pointer) among the sender(s) and the one receiver
    // in a mpsc setting
    struct SharedInner<T> {
        inner_mut_data: Mutex<SharedInnerMut<T>>,
        recv_wakeup_flag: Condvar,
    }

    // modelling the data parts, within the the common entity as above, that both sender(s) and receiver parties
    // would mutate, synchronized by Mutex in this implementation
    struct SharedInnerMut<T> {
        msg_queue: VecDeque<T>,
        // these fields are to keep a correct account of number of sender and the presence of receiver
        // in the mpsc setup. the reference count to the common shared entity of the channel cannot naively
        // give the accurate account, for example, when querying whether there is any senders left, a reference
        // count of 1 wouldn't tell whether that's 1 sender or receiver left alive
        sender_cnt: usize,
        receiver_live: bool,
    }

    impl<T> SharedInnerMut<T> {
        // provide utility to intialize such structured, ready to be called by public-facing API for creating new channel
        fn new() -> Self {
            Self {
                msg_queue: VecDeque::new(),
                sender_cnt: 1,
                receiver_live: true,
            }
        }
    }
    
    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        
        let new_shared_inner = Arc::new(SharedInner {
            inner_mut_data: Mutex::new(SharedInnerMut::new()),
            recv_wakeup_flag: Condvar::new(),
        });
        
        ( 
            Sender { shared_inner: Arc::clone(&new_shared_inner) },
            Receiver { shared_inner: Arc::clone(&new_shared_inner) },
        )
    }
}



#[cfg(test)]
mod tests{
    use std::{thread, time::Duration};

    use super::*;
    #[test]
    fn channel_only_channel_basic_send_recv() {
        todo!()
    }
    #[test]
    fn tx_rx_channel_naive_send_recv() {
        let (test_tx, test_rx) = tx_rx_channel::channel::<u32>();
        let _ = test_tx.send(42);
        assert_eq!(test_rx.tx_unaware_recv(), 42);
        // would hang forever due to the buggy implementation of recv s.t.
        // wherein there is no mechanism to notify the thread executing the recv call that is blocked
        // on waiting for the cond var, in the circumstance of no msg and no senders left, that the
        // blocked call should just return
        // test_rx.tx_unaware_recv();   
    }

    /// impl test cases to ensure correct erroneous situations are handled when making send or recv calls
    #[test]
    fn rx_err_for_no_tx_before_blocking() {
        let (test_tx, test_rx) = tx_rx_channel::channel::<u32>();
        let _ = test_tx.send(42);
        assert_eq!(test_rx.recv().unwrap(), 42);
        // a case showing the awareness of the recv call s.t. if all senders are gone and no msg is to receive
        // the call would return err immediately
        drop(test_tx);
        assert!(test_rx.recv().is_err())
    }

    #[test]
    fn rx_err_for_no_tx_while_blocking() {
        let (test_tx, test_rx) = tx_rx_channel::channel::<u32>();

        thread::scope(|scope|
            {   
                // spawn a background thread that would take and drop the sender after some delay,
                // while the main thread is blocked in the recv call execution that is waiting on
                // more msgs to receive
                scope.spawn(move || {
                    thread::sleep(Duration::from_secs(3));
                    drop(test_tx);
                });
            });
        
        // on the main thread, the recv call initially blocks on waiting for more msgs to receive
        // but eventually return properly when there is no possibilty to do so (no msg & no sender)
        assert!(test_rx.recv().is_err());
    }

    #[test]
    fn tx_err_for_no_rx() {
        let (test_tx, test_rx) = tx_rx_channel::channel::<u32>();
        drop(test_rx);
        assert_eq!(test_tx.send(42).unwrap_err().0, 42);
    }
}