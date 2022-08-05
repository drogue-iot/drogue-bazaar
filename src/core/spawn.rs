use core::{future::Future, pin::Pin};

/// A spawner, gathering spawned tasks during the setup phase.
///
/// NOTE: The spawner itself might not execute/drive those tasks. It may be necessary to hand over
/// gathered tasks to some method like [`run_main`].
pub trait Spawner {
    fn spawn(&mut self, future: Pin<Box<dyn Future<Output = anyhow::Result<()>>>>);
}

pub trait SpawnerExt {
    fn spawn_iter<I>(self, iter: I)
    where
        I: IntoIterator<Item = Pin<Box<dyn Future<Output = anyhow::Result<()>>>>>;
}

impl SpawnerExt for &mut dyn Spawner {
    fn spawn_iter<I>(self, iter: I)
    where
        I: IntoIterator<Item = Pin<Box<dyn Future<Output = anyhow::Result<()>>>>>,
    {
        for i in iter {
            self.spawn(i);
        }
    }
}

impl Spawner for Vec<futures_core::future::LocalBoxFuture<'_, anyhow::Result<()>>> {
    fn spawn(&mut self, future: Pin<Box<dyn Future<Output = anyhow::Result<()>>>>) {
        self.push(future);
    }
}
