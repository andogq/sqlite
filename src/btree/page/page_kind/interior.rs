use std::marker::PhantomData;

use derive_more::Deref;
use zerocopy::TryFromBytes;

use crate::{
    btree::page::{Page, PageCommon, PageExt, PageFlag, PageType, disk::DiskInteriorPageHeader},
    pager::PageBuffer,
};

use super::PageKindFlag;

#[derive(Clone, Debug, Deref)]
pub struct InteriorPage<T: PageType> {
    #[deref]
    common: PageCommon<T>,

    /// Pointer to the right most page.
    pub right_pointer: u32,
}

impl<T: PageType> PageExt<T> for InteriorPage<T> {
    fn from_buffer(buffer: PageBuffer) -> Self {
        let header = DiskInteriorPageHeader::try_ref_from_bytes(&buffer).unwrap();

        let Some(flag) = PageFlag::new(header.flag).filter(|flag| {
            matches!(flag.kind_flag, PageKindFlag::Interior) && flag.type_flag.is::<T>()
        }) else {
            panic!("invalid page flag in header: {}", header.flag);
        };

        Self {
            right_pointer: header.right_page_pointer.get(),
            common: PageCommon {
                flag,
                first_freeblock: PageCommon::<T>::first_freeblock(header.first_freeblock),
                cell_count: PageCommon::<T>::cell_count(header.cell_count),
                cell_content_area_offset: PageCommon::<T>::cell_content_area_offset(
                    header.cell_content_area_offset,
                ),
                free_bytes: header.fragmented_free_bytes_count,
                buffer,
                page_type: PhantomData,
            },
        }
    }

    fn to_page(self) -> Page<T> {
        Page::Interior(self)
    }
}
