use std::num::NonZeroUsize;

use zerocopy::{FromBytes, big_endian::U32};

use super::{
    MemoryPage,
    pager::{PageId, Pager},
};

#[derive(Clone)]
pub struct Chain {
    pager: Pager,
    current_page: Option<MemoryPage>,
    next_page: Option<PageId>,
}

impl Chain {
    pub fn new(pager: Pager, page: MemoryPage, next_page: Option<PageId>) -> Self {
        Self {
            pager,
            current_page: Some(page),
            next_page,
        }
    }

    fn pages(&self) -> impl Iterator<Item = MemoryPage> {
        let mut current_page = self.current_page.clone();
        let mut next_page_id = self.next_page;

        std::iter::from_fn(move || {
            if let Some(current_page) = current_page.take() {
                return Some(current_page.clone());
            }

            if let Some(page_id) = next_page_id.take() {
                let page = self.pager.get(page_id).unwrap().unwrap();

                next_page_id = NonZeroUsize::new(
                    U32::ref_from_bytes(&page.slice(0..4).buffer())
                        .unwrap()
                        .get() as usize,
                )
                .map(PageId::new);

                return Some(page);
            }

            None
        })
    }

    pub fn copy_to_slice(&self, offset: usize, buf: &mut [u8]) {
        let mut pos = 0;

        let pages = self
            .pages()
            .skip_while(|p| {
                let len = p.buffer().len();
                if pos + len < offset {
                    pos += len;
                    return true;
                }

                false
            })
            .collect::<Vec<_>>();

        let mut b = buf;
        for (i, mut page) in pages.into_iter().enumerate() {
            if i == 0 {
                // Fix the bounds of the first slice
                page = page.slice(offset - pos..);
            }

            let data = page.buffer();

            if b.len() > data.len() {
                // Select the byte range to be written to.
                let (target, rest) = b.split_at_mut(data.len());
                b = rest;

                target.copy_from_slice(&data);
            } else {
                // Fill the rest of the buffer.
                b.copy_from_slice(&data[..b.len()]);

                return;
            }
        }

        panic!("not enough bytes to copy");
    }
}
