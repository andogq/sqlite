mod btree;
mod command;
mod ctx;
mod disk;
mod record;

use std::fs::File;

use self::btree::page::{Page, PageExt, Table};
use command::{CreateStatement, QueryStatement};
use ctx::Ctx;
use record::Record;

const DATABASE: &str = "test.db";
const COMMAND: &str = "select * from users;";

#[allow(unused)]
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
                let mut payload = vec![0; cell.payload.length];
                cell.payload.copy_to_slice(ctx.clone(), &mut payload);

                DatabaseSchema::from(Record::from_buf(cell.row_id, &payload))
            })
            .collect::<Vec<_>>()
    };

    let command = command::parse_command::<QueryStatement>(COMMAND);

    let schema = schemas
        .iter()
        .find(|schema| schema.name == *command.table_name)
        .unwrap();

    let columns = command::parse_command::<CreateStatement>(&schema.sql.to_lowercase())
        .columns
        .into_iter()
        .collect::<Vec<_>>();

    let page = Page::<Table>::from_buffer(ctx.pager.get_page(schema.root_page));
    btree::traverse(ctx.clone(), page)
        .map(|cell| {
            let mut payload = vec![0; cell.payload.length];
            cell.payload.copy_to_slice(ctx.clone(), &mut payload);

            Record::from_buf(cell.row_id, &payload)
        })
        .for_each(|record| {
            columns.iter().zip(record.fields).for_each(|(col, value)| {
                println!("{} ({}): {:?}", *col.column_name, *col.type_name, value);
            });
            println!();
        })
}
