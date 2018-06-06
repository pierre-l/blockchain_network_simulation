use futures::Future;
use futures::future;
use futures::Stream;
use futures::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
pub use network::transport::{MPSCConnection, send_or_panic};
use network::transport::MPSCAddress;
use network::transport::MPSCTransport;
use rand::{self, Rng};
use std::collections::HashSet;
use std::hash::Hash;
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use tokio;

pub trait Node<M>{
    fn on_new_connection(&self, connection: MPSCConnection<M>);
    fn on_start(&mut self);
}

pub mod transport;

pub struct Network<M> where M: Clone + Send + 'static{
    transports: Vec<MPSCTransport<M>>,
}

impl <M> Network<M> where M: Clone + Send + 'static{
    pub fn new(size: usize, average_number_of_connections_per_node: usize)
        -> Network<M> where M: Clone + Send + 'static
    {
        let mut transports = vec![];
        let mut addresses = vec![];
        let mut defined_connections = BiSet::new();

        for i in 0..size {
            let node = MPSCTransport::new(i);
            addresses.push(node.address().clone());
            transports.push(node);
        }

        for transports in &mut transports{
            let mut candidate_addresses = vec![];

            let node_address_id = *transports.address().id();
            for candidate in &addresses{
                let candidate_address_id = *candidate.id();
                if node_address_id != candidate_address_id
                    && !defined_connections.contains(node_address_id, candidate_address_id)
                {
                    candidate_addresses.push(candidate.clone());
                }
            }

            for _i in 0..average_number_of_connections_per_node/2 + 1 {
                let pool_not_empty = candidate_addresses.len() > 0;
                if pool_not_empty {
                    let seed_index = transports.random_different_address(&candidate_addresses);

                    let seed_address = candidate_addresses.remove(seed_index);
                    defined_connections.insert(*seed_address.id(), node_address_id);
                    transports.include_seed(seed_address);
                }
            }
        }

        Network{
            transports
        }
    }

    pub fn run<N, F>(self, node_factory: F)
        where
            N: Node<M> + Sync + Send + 'static,
            F: Fn() -> N + Send + 'static
    {
        let nodes = self.transports;
        let handle = thread::spawn(move ||{
            let (sender, receiver) = stream_of(nodes);
            let nodes_future = receiver
                .for_each(move |transport|{
                    info!("Starting a new node.");
                    let mut node = node_factory();
                    node.on_start();

                    let node = Arc::new(node);

                    let node_future = transport.run()
                        .for_each(move |connection|{
                            node.on_new_connection(connection);
                            future::ok(())
                        })
                        .then(|_|{
                            info!("Node stopped.");
                            future::ok(())
                        })
                        .map_err(|()|{});

                    tokio::spawn(node_future);
                    future::ok(())
                })
            ;

            tokio::run(nodes_future);

            drop(sender);
        });

        thread::sleep(Duration::from_millis(60000));

        drop(handle);
    }
}

fn stream_of<T>(vector: Vec<T>) -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    let (sender, receiver,) = mpsc::unbounded::<T>();

    for item in vector{
        transport::send_or_panic(&sender, item);
    }

    (sender, receiver,)
}

impl <M> MPSCTransport<M> where M: Clone + Send + 'static{
    fn random_different_address(&self, pool: &Vec<MPSCAddress<M>>) -> usize{
        let mut rng = rand::thread_rng();
        rng.gen_range(0, pool.len())
    }
}

/// A very naive HashSet for tuples.
/// May not be the most efficient because 'contains' method instantiate a new tuple, requiring
/// owned items.
struct BiSet<T> where T: Hash + Ord{
    inner: HashSet<(T, T)>
}

impl <T> BiSet<T> where T: Hash + Ord{
    pub fn new() -> BiSet<T>{
        BiSet{
            inner: HashSet::new()
        }
    }

    pub fn insert(&mut self, one: T, other: T) {
        if one < other {
            self.inner.insert((one, other));
        } else {
            self.inner.insert((other, one));
        }
    }

    pub fn contains(&self, one: T, other: T) -> bool{
        if one < other {
            self.inner.contains(&(one, other))
        } else {
            self.inner.contains(&(other, one))
        }
    }
}


#[cfg(test)]
mod tests{
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
    use std::time::Duration;
    use super::*;

    #[derive(Clone, Debug)]
    pub struct Message{}

    pub struct TestNode{
        received_messages: Arc<AtomicUsize>,
        notified_of_start: Arc<AtomicBool>,
    }

    impl Node<Message> for TestNode{
        fn on_new_connection(&self, connection: MPSCConnection<Message>) {
            let received_messages = self.received_messages.clone();
            let (sender, receiver) = connection.split();

            // Send one message per connection received for each node.
            send_or_panic(&sender, Message{});

            let reception = receiver
                .for_each(move |_message|{
                    println!("Message received.");
                    received_messages.fetch_add(1, Ordering::Relaxed);
                    future::ok(())
                })
                .map_err(|_|{
                    panic!()
                });
            tokio::spawn(reception);
        }

        fn on_start(&mut self) {
            self.notified_of_start.store(true, Ordering::Relaxed)
        }
    }

    #[test]
    fn can_create_a_network(){
        let network = Network::new(4, 1);

        let global_number_of_received_messages = Arc::new(AtomicUsize::new(0));
        let notified_of_start = Arc::new(AtomicBool::new(false));

        let received_messages_clone = global_number_of_received_messages.clone();
        let notified_of_start_clone = notified_of_start.clone();
        network.run(move ||{
            TestNode{
                received_messages: received_messages_clone.clone(),
                notified_of_start: notified_of_start_clone.clone(),
            }
        });

        thread::sleep(Duration::from_millis(2000));
        assert_eq!(8, global_number_of_received_messages.load(Ordering::Relaxed));
        assert!(notified_of_start.load(Ordering::Relaxed));
    }
}