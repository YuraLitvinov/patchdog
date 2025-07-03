use std::collections::{HashMap,
    HashSet,
    VecDeque};
use std::fmt;

// === Enums ===
#[derive(Debug)]
enum Status {
    Active,
    Inactive,
    Unknown,
}

#[derive(Debug)]
enum ItemType {
    Book,
    Gadget,
    Clothing,
}

// === Structs ===
#[derive(Debug)]
struct Item {
    name: String,
    item_type: ItemType,
    price: f32,
    status: Status,
}

#[derive(Debug)]
struct Shelf {
    label: String,
    items: Vec<Item>,
}

#[derive(Debug)]
struct Warehouse {
    shelves: HashMap<String, Shelf>,
}


// === Implementations ===
impl Item {
    fn new(
        name: &str,
        item_type: ItemType,
        price: f32,
    ) -> Self {
        Self {
            name: name.to_string(),
            item_type,
            price,
            status: Status::Active,
        }
    }

    fn deactivate(
        &mut self
    ) {
        self.status = Status::Inactive;
    }
}

impl Shelf {
    fn new(
        label: &str
    ) -> Self {
        Self {
            label: label.to_string(),
            items: Vec::new(),
        }
    }

    fn add_item(
        &mut self,
        item: Item,
    ) {
        self.items.push(item);
    }

    fn list_active_items(
        &self
    ) {
        println!("Items on shelf '{}':", self.label);
        for item in &self.items {
            if let Status::Active = item.status {
                println!(
                    " - {} ({:?}): ${}",
                    item.name, item.item_type, item.price
                );
            }
        }
    }
}

impl Warehouse {
    fn new() -> Self {
        Self {
            shelves: HashMap::new(),
        }
    }

    fn add_shelf(
        &mut self,
        shelf: Shelf,
    ) {
        self.shelves.insert(shelf.label.clone(), shelf);
    }

    fn list_all_items(
        &self
    ) {
        for (label, shelf) in &self.shelves {
            println!("\nShelf: {}", label);
            shelf.list_active_items();
        }
    }
}

// === Sample function with 3+ objects ("shelf-like" grouping) ===
fn setup_warehouse() -> Warehouse {
    let mut shelf1 = Shelf::new("A1");
    shelf1.add_item(Item::new(
        "Rust Book",
        ItemType::Book,
        29.99,
    ));
    shelf1.add_item(Item::new(
        "Bluetooth Speaker",
        ItemType::Gadget,
        49.99,
    ));
    shelf1.add_item(Item::new(
        "T-Shirt",
        ItemType::Clothing,
        19.99,
    ));

    let mut shelf2 = Shelf::new("B2");
    shelf2.add_item(Item::new(
        "Keyboard",
        ItemType::Gadget,
        79.99,
    ));
    shelf2.add_item(Item::new(
        "Novel",
        ItemType::Book,
        14.99,
    ));
    shelf2.add_item(Item::new(
        "Jeans",
        ItemType::Clothing,
        39.99,
    ));

    let mut warehouse = Warehouse::new();
    warehouse.add_shelf(shelf1);
    warehouse.add_shelf(shelf2);

    warehouse
}
fn bookshop(
    name: &str,
    item_type: ItemType,
    price: f32,
) -> Self {
    Self {
        name: name.to_string(),
        item_type,
        price,
        status: Status::Active,
    }
}

// === Entry Point ===
fn main() {
    let warehouse = setup_warehouse();
    warehouse.list_all_items();
}