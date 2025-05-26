mod btree;
mod ctx;
mod disk;

use std::fs::File;

use self::{
    btree::page::{Page, PageExt, Table},
    disk::var_int::VarInt,
};
use ctx::Ctx;

const DATABASE: &str = "test.db";

fn main() {
    let file = File::open(DATABASE).unwrap();
    let ctx = Ctx::new(file);

    {
        // Read the first page into memory.
        let root_page = ctx.pager.get_page(0);

        let page = Page::<Table>::from_buffer(root_page);
        dbg!(page.cell_count);

        btree::traverse(ctx.clone(), page).for_each(move |cell| {
            println!(
                "row id: {}, payload length: {}",
                cell.row_id, cell.payload.length
            );

            let mut payload = vec![0; cell.payload.length];
            cell.payload.copy_to_slice(ctx.clone(), &mut payload);

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
