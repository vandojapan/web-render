use std::{collections::HashMap, sync::Mutex};

enum Buffer {
    Rendering,
    Ready(Box<[u8]>),
}

/// One outstanding image per effect. Cache invalidation must never clear these leases.
#[derive(Default)]
pub struct ImageBuffers(Mutex<HashMap<i32, Buffer>>);

impl ImageBuffers {
    pub fn reserve(&self, id: i32) -> anyhow::Result<Reservation<'_>> {
        let mut buffers = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Image buffer lock poisoned"))?;
        if buffers.contains_key(&id) {
            anyhow::bail!(
                "Effect {id} already has an outstanding image; free it before rendering again"
            );
        }
        buffers.insert(id, Buffer::Rendering);
        Ok(Reservation {
            owner: self,
            id,
            committed: false,
        })
    }

    pub fn free(&self, id: i32) -> bool {
        let Ok(mut buffers) = self.0.lock() else {
            return false;
        };
        // A premature free cannot cancel a reservation or allow a competing render.
        if matches!(buffers.get(&id), Some(Buffer::Ready(_))) {
            buffers.remove(&id);
            true
        } else {
            false
        }
    }
}

pub struct Reservation<'a> {
    owner: &'a ImageBuffers,
    id: i32,
    committed: bool,
}

impl Reservation<'_> {
    pub fn commit(mut self, image: Vec<u8>) -> anyhow::Result<*const u8> {
        let mut buffers = self
            .owner
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Image buffer lock poisoned"))?;
        buffers.insert(self.id, Buffer::Ready(image.into_boxed_slice()));
        let Some(Buffer::Ready(image)) = buffers.get(&self.id) else {
            unreachable!()
        };
        let pointer = image.as_ptr();
        self.committed = true;
        Ok(pointer)
    }
}

impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        if !self.committed
            && let Ok(mut buffers) = self.owner.0.lock()
        {
            buffers.remove(&self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returned_pointer_is_owned_until_free() {
        let buffers = ImageBuffers::default();
        let ptr = buffers
            .reserve(1)
            .unwrap()
            .commit(vec![7, 8, 9, 255])
            .unwrap();
        assert!(buffers.reserve(1).is_err());
        // SAFETY: this reservation owns four initialized bytes and has not been freed.
        assert_eq!(
            unsafe { std::slice::from_raw_parts(ptr, 4) },
            &[7, 8, 9, 255]
        );
        assert!(buffers.free(1));
        assert!(!buffers.free(1));
        assert!(buffers.reserve(1).is_ok());
    }

    #[test]
    fn failed_render_and_premature_free_do_not_corrupt_reservations() {
        let buffers = ImageBuffers::default();
        let reservation = buffers.reserve(1).unwrap();
        assert!(!buffers.free(1));
        assert!(buffers.reserve(1).is_err());
        assert!(buffers.reserve(2).is_ok());
        drop(reservation);
        assert!(buffers.reserve(1).is_ok());
    }
}
