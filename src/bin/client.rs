use bytes::Bytes;
use mini_redis::client;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

/// Multiple different commands are multiplexed over a single channel.
#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    },
}

/// Provided by the requester and used by the manager task to send
/// the command response back to the requester.
type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[tokio::main]
async fn main() {
    // Create a new channel with a capacity of at most 32
    let (tx, mut rx) = mpsc::channel(32);
    // let (tx, rx) = oneshot::channel();
    let tx2 = tx.clone();

    // tokio::spawn(async move {
    //     tx.send("Sending from first handle").await.unwrap();
    // });

    // tokio::spawn(async move {
    //     tx2.send("Sending from second handle").await.unwrap();
    // });

    // while let Some(message) = rx.recv().await {
    //     println!("Received : {message}");
    // }

    let manager = tokio::spawn(async move {
        // Establish a connection to the server
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        // Start receiving messages
        while let Some(cmd) = rx.recv().await {
            use Command::*;

            match cmd {
                Get { key, resp } => {
                    let res = client.get(&key).await;
                    // Ignore errors
                    let _ = resp.send(res);
                }
                Set { key, val, resp } => {
                    let res = client.set(&key, val).await;
                    // Ignore erros
                    let _ = resp.send(res);
                }
            }
        }
    });

    // Spawn two tasks, one gets a key and the other sets a key
    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "foo".to_string(),
            resp: resp_tx,
        };

        // Send the GET request
        tx.send(cmd).await.unwrap();

        let res = resp_rx.await;
        println!("Received: {res:?}");
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "foo".to_string(),
            val: "bar".into(),
            resp: resp_tx,
        };

        // Send the SET request
        tx2.send(cmd).await.unwrap();

        // Await the response
        let res = resp_rx.await;
        println!("Received: {res:?}");
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}
