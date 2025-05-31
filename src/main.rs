mod btree;
mod command;
mod ctx;
mod disk;
mod record;

use std::fs::File;

use self::btree::page::{Page, PageExt, Table};
use ctx::Ctx;
use record::Record;

const DATABASE: &str = "test.db";

#[derive(Clone, Debug)]
struct DatabaseSchema {
    r#type: String,
    name: String,
    tbl_name: String,
    root_page: u32,
    sql: String,
}

impl From<Record> for DatabaseSchema {
    fn from(record: Record) -> Self {
        let mut fields = record.fields.into_iter();

        Self {
            r#type: fields.next().unwrap().string().unwrap(),
            name: fields.next().unwrap().string().unwrap(),
            tbl_name: fields.next().unwrap().string().unwrap(),
            root_page: fields.next().unwrap().integer().unwrap() as u32,
            sql: fields.next().unwrap().string().unwrap(),
        }
    }
}

fn main() {
    let file = File::open(DATABASE).unwrap();
    let ctx = Ctx::new(file);

    let schemas = {
        // Read the first page into memory.
        let root_page = ctx.pager.get_page(1);

        let page = Page::<Table>::from_buffer(root_page);

        btree::traverse(ctx.clone(), page)
            .map(|cell| {
                println!(
                    "row id: {}, payload length: {}",
                    cell.row_id, cell.payload.length
                );

                let mut payload = vec![0; cell.payload.length];
                cell.payload.copy_to_slice(ctx.clone(), &mut payload);

                DatabaseSchema::from(Record::from_buf(cell.row_id, &payload))
            })
            .collect::<Vec<_>>()
    };

    for schema in schemas {
        dbg!(&schema);

        let page = Page::<Table>::from_buffer(ctx.pager.get_page(schema.root_page));
        btree::traverse(ctx.clone(), page)
            .map(|cell| {
                println!(
                    "row id: {}, payload length: {}",
                    cell.row_id, cell.payload.length
                );

                let mut payload = vec![0; cell.payload.length];
                cell.payload.copy_to_slice(ctx.clone(), &mut payload);

                Record::from_buf(cell.row_id, &payload)
            })
            .for_each(|record| {
                dbg!(record);
            })
    }

    command::do_something();
}
