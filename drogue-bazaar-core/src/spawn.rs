use core::{future::Future, pin::Pin};

/// A spawner, gathering spawned tasks during the setup phase.
///
/// NOTE: The spawner itself might not execute/drive those tasks. It may be necessary to hand over
/// gathered tasks to some method like [`run_main`].
pub trait Spawner<O> {
    fn spawn(&mut self, future: Pin<Box<dyn Future<Output = O>>>);
}

#[cfg(feature = "alloc")]
impl<O> Spawner<O> for Vec<futures_core::future::LocalBoxFuture<'_, O>> {
    fn spawn(&mut self, future: Pin<Box<dyn Future<Output = O>>>) {
        self.push(future);
    }
}
