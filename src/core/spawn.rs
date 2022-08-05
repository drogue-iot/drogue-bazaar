use core::{future::Future, pin::Pin};

/// A spawner, gathering spawned tasks during the setup phase.
///
/// NOTE: The spawner itself might not execute/drive those tasks. It may be necessary to hand over
/// gathered tasks to some method like [`run_main`].
pub trait Spawner {
    fn spawn_boxed(&mut self, future: Pin<Box<dyn Future<Output = anyhow::Result<()>>>>);
}

pub trait SpawnerExt: Spawner {
    fn spawn_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Pin<Box<dyn Future<Output = anyhow::Result<()>>>>>,
    {
        for i in iter {
            self.spawn_boxed(i);
        }
    }

    fn spawn<F>(&mut self, f: F)
    where
        F: Future<Output = anyhow::Result<()>> + 'static,
    {
        self.spawn_boxed(Box::pin(f))
    }
}

impl<S: ?Sized> SpawnerExt for S where S: Spawner {}

impl Spawner for Vec<futures_core::future::LocalBoxFuture<'_, anyhow::Result<()>>> {
    fn spawn_boxed(&mut self, future: Pin<Box<dyn Future<Output = anyhow::Result<()>>>>) {
        self.push(future);
    }
}
