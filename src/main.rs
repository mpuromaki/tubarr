static DB_PATH: &'static str = "./db.sqlite";

mod database;

fn main() {
    let dbp = database::init_from(DB_PATH);
}
