mod shardeddata;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub use shardeddata::ShardedLockedData;

pub struct AsyncSleepFuture {
    start: Instant,
    dur: Duration,
}

impl std::future::Future for AsyncSleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.start.elapsed() >= self.dur {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

pub fn sleep(dur: Duration) -> AsyncSleepFuture {
    AsyncSleepFuture {
        dur,
        start: std::time::Instant::now(),
    }
}
