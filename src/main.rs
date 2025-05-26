mod btree;
mod disk;
mod pager;

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    iter,
};

use self::{
    btree::{
        page::{Page, PageExt, Table},
        payload::Payload,
    },
    disk::{header::SqliteHeader, var_int::VarInt},
    pager::Pager,
};
use zerocopy::{FromBytes, big_endian::*};

const DATABASE: &str = "test.db";

fn traverse(ctx: DbCtx, page: Page<Table>, pager: Pager) -> impl Iterator<Item = TableCell> {
    let mut stack = vec![page];
    let mut leaf_iter = None;

    std::iter::from_fn(move || {
        match &mut leaf_iter {
            None => {
                match stack.pop()? {
                    Page::Leaf(leaf_page) => {
                        // Buffer all of the pointers into a vec, so they can be referred to from
                        // the iterator.
                        let ptrs = leaf_page.cell_content_pointers().collect::<Vec<_>>();
                        let ctx = ctx.clone();

                        leaf_iter = Some(ptrs.into_iter().map(move |ptr| {
                            let cell_content = &leaf_page.cell_content_area()[ptr..];
                            let (payload_size, buf) = VarInt::from_buffer(cell_content);
                            let (row_id, payload) = VarInt::from_buffer(buf);

                            let payload_offset = ptr + (cell_content.len() - payload.len());

                            // TODO: Trim payload and account for overflow

                            TableCell {
                                row_id: *row_id,
                                payload: Payload::from_buf_with_payload_size(
                                    ctx.clone(),
                                    leaf_page.clone().to_page(),
                                    payload_offset,
                                    *payload_size as usize,
                                ),
                            }
                        }));
                    }
                    Page::Interior(interior_page) => {
                        // Capture the current end of the array, so later pages don't jump ahead.
                        let insert_point = stack.len();

                        interior_page
                            .cell_content_pointers()
                            .map({
                                let cell_content = interior_page.cell_content_area();
                                |ptr| &cell_content[ptr..]
                            })
                            .map(|cell_content| {
                                let (left_pointer, _cell_content) =
                                    U32::read_from_prefix(cell_content).unwrap();
                                left_pointer.get()
                            })
                            .chain(iter::once(interior_page.right_pointer))
                            .for_each(|ptr| {
                                stack.insert(insert_point, Page::from_buffer(pager.get_page(ptr)));
                            });
                    }
                }
            }
            Some(iter) => {
                if let Some(next) = iter.next() {
                    return Some(Some(next));
                }
                leaf_iter = None;
            }
        }

        Some(None)
    })
    .flatten()
}

struct TableCell {
    row_id: i64,
    payload: Payload<Table>,
}

#[derive(Clone, Debug)]
struct DbCtx {
    pub page_size: usize,
    pub page_end_padding: usize,
}

impl From<&disk::header::SqliteHeader> for DbCtx {
    fn from(header: &disk::header::SqliteHeader) -> Self {
        Self {
            page_size: header.page_size() as usize,
            page_end_padding: header.page_end_padding() as usize,
        }
    }
}

fn main() {
    let mut file = File::open(DATABASE).unwrap();

    let ctx = {
        let mut header_buf = [0; 100];
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_exact(&mut header_buf).unwrap();
        let header = SqliteHeader::read_from_buffer(&header_buf).unwrap();

        DbCtx::from(header)
    };

    let pager = Pager::new(file);

    {
        // Read the first page into memory.
        let root_page = pager.get_page(0);

        let page = Page::<Table>::from_buffer(root_page);
        dbg!(page.cell_count);

        traverse(ctx, page, pager.clone()).for_each(move |cell| {
            println!(
                "row id: {}, payload length: {}",
                cell.row_id, cell.payload.length
            );

            let mut payload = vec![0; cell.payload.length];
            cell.payload.copy_to_slice(pager.clone(), &mut payload);

            let (header_length, buf) = VarInt::from_buffer(&payload);
            let remaining_header = *header_length as usize - (payload.len() - buf.len());

            let mut buf = &buf[..remaining_header];

            while !buf.is_empty() {
                let (serial_type, rest) = VarInt::from_buffer(buf);
                buf = rest;

                println!(
                    "{}",
                    match *serial_type {
                        0 => "NULL",
                        1 => "i8",
                        2 => "i16",
                        3 => "i24",
                        4 => "i32",
                        5 => "i48",
                        6 => "i64",
                        7 => "f64",
                        8 => "0",
                        9 => "1",
                        10 | 11 => "reserved",
                        n @ 12.. if n % 2 == 0 => "BLOB",
                        n @ 13.. if n % 2 == 1 => "text",
                        _ => unreachable!(),
                    }
                );
            }
        });
    }
}
