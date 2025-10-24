#[cfg(test)]
mod tests_vol1 {
    use anyhow::Result;
    use futures_util::future::join_all;
    use std::{
        fs::{File, OpenOptions},
        io::Write,
        sync::{Arc, Mutex},
        task::{Poll, Waker},
        time::Duration,
    };
    use tokio::{
        sync::mpsc,
        task::{self, JoinHandle},
        time::*,
    };

    async fn slow_task() -> &'static str {
        sleep(Duration::from_secs(10)).await;
        "slow task completed"
    }

    #[tokio::test]
    async fn test_1() {
        let duration = Duration::from_secs(3);
        let result = timeout(duration, slow_task()).await;
        match result {
            Ok(v) => println!("task succeed: {}", v),
            Err(_) => println!("task time out"),
        }
    }

    struct MyFutureState {
        data: Option<Vec<u8>>,
        waker: Option<Waker>,
    }

    struct MyFuture {
        state: Arc<Mutex<MyFutureState>>,
    }

    impl MyFuture {
        fn new() -> (Self, Arc<Mutex<MyFutureState>>) {
            let state = Arc::new(Mutex::new(MyFutureState {
                data: None,
                waker: None,
            }));
            (
                MyFuture {
                    state: state.clone(),
                },
                state,
            )
        }
    }

    impl Future for MyFuture {
        type Output = String;

        fn poll(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            println!("polling the future");
            let mut state = self.state.lock().unwrap();
            if state.data.is_some() {
                let data = state.data.take().unwrap();
                Poll::Ready(String::from_utf8(data).unwrap())
            } else {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    #[tokio::test]
    async fn test_2() {
        let (my_future, state) = MyFuture::new();
        let (tx, mut rx) = mpsc::channel::<()>(1);
        let task_handler = task::spawn(async { my_future.await });
        sleep(Duration::from_secs(3)).await;
        println!("spawning trigger task");

        let trigger_task = task::spawn(async move {
            rx.recv().await;
            let mut state = state.lock().unwrap();
            state.data = Some(b"hello from the outside".to_vec());
            loop {
                if let Some(waker) = state.waker.take() {
                    waker.wake();
                    break;
                }
            }
        });
        tx.send(()).await.unwrap();

        let outcome = task_handler.await.unwrap();
        println!("task completed with outcome: {}", outcome);
        trigger_task.await.unwrap();
    }

    type AsyncFileHandle = Arc<Mutex<File>>;
    type FileJoinHandle = JoinHandle<Result<bool>>;

    struct AsyncWriteFuture {
        pub handle: AsyncFileHandle,
        pub entry: String,
    }

    impl Future for AsyncWriteFuture {
        type Output = Result<bool>;

        fn poll(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<Self::Output> {
            let mut guard = match self.handle.try_lock() {
                Ok(guard) => guard,
                Err(error) => {
                    println!("error for {}: {}", self.entry, error);
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
            };
            let lined_entry = format!("{}\n", self.entry);

            match guard.write_all(lined_entry.as_bytes()) {
                Ok(_) => println!("written for: {}", self.entry),
                Err(e) => println!("{}", e),
            };
            Poll::Ready(Ok(true))
        }
    }

    fn get_handle(file_path: &dyn ToString) -> AsyncFileHandle {
        match OpenOptions::new().append(true).open(file_path.to_string()) {
            Ok(file) => Arc::new(Mutex::new(file)),
            Err(_) => Arc::new(Mutex::new(File::create(file_path.to_string()).unwrap())),
        }
    }

    fn write_log(file_handle: AsyncFileHandle, line: String) -> FileJoinHandle {
        let future = AsyncWriteFuture {
            handle: file_handle,
            entry: line,
        };
        task::spawn(async move { future.await })
    }

    #[tokio::test]
    async fn test_3() {
        let login_handle = get_handle(&"login.txt");
        let logout_handle = get_handle(&"logout.txt");
        let names = ["one", "two", "three", "four", "five"];
        let mut handles = Vec::new();
        for name in names {
            let login = login_handle.clone();
            let logout = logout_handle.clone();
            let handle = write_log(login, name.to_owned());
            let handle_2 = write_log(logout, name.to_owned());

            handles.push(handle);
            handles.push(handle_2);
        }

        let _ = join_all(handles).await;
    }
}
