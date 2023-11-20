#[macro_use]
extern crate serde;

use candid::{CandidType, Decode, Encode, Principal};
use ic_cdk::{api::time, caller};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable,
};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(CandidType, Clone, Serialize, Deserialize, Default)]
struct Event {
    id: u64,
    event_description: String,
    owner: Principal,
    event_title: String,
    event_location: String,
    event_card_imgurl: String,
    attendees: Vec<Principal>,
    created_at: u64,
    updated_at: Option<u64>,
}

impl Storable for Event {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Event {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Event, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        ));
}

#[derive(CandidType, Serialize, Deserialize, Default)]
struct EventPayload {
    event_description: String,
    event_title: String,
    event_location: String,
    event_card_imgurl: String,
}

#[ic_cdk::query]
fn get_event(id: u64) -> Result<Event, Error> {
    match _get_event(&id) {
        Some(event) => Ok(event),
        None => Err(Error::NotFound(format!("Event with id={} not found", id))),
    }
}

#[ic_cdk::update]
fn create_event(payload: EventPayload) -> Option<Event> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");

    let event = Event {
        id,
        event_description: payload.event_description,
        owner: caller(),
        event_title: payload.event_title,
        event_location: payload.event_location,
        event_card_imgurl: payload.event_card_imgurl,
        attendees: Vec::new(),
        created_at: time(),
        updated_at: None,
    };

    do_insert(&event);
    Some(event)
}

#[ic_cdk::update]
fn update_event(id: u64, payload: EventPayload) -> Result<Event, Error> {
    let event = match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut event) => {
            validate_owner(&event)?;
            event.event_description = payload.event_description;
            event.event_title = payload.event_title;
            event.event_location = payload.event_location;
            event.event_card_imgurl = payload.event_card_imgurl;
            event.updated_at = Some(time());
            event
        }
        None => return Err(Error::NotFound(format!("Event with id={} not found", id))),
    };

    do_insert(&event);
    Ok(event)
}

#[ic_cdk::update]
fn attend_event(id: u64) -> Result<Event, Error> {
    let mut event = match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut event) => {
            let caller_principal = caller();
            if event.attendees.contains(&caller_principal) {
                return Err(Error::AlreadyAttending);
            }
            event.attendees.push(caller_principal);
            event
        }
        None => return Err(Error::NotFound(format!("Event with id={} not found", id))),
    };

    do_insert(&event);
    Ok(event)
}

#[ic_cdk::update]
fn delete_event(id: u64) -> Result<Event, Error> {
    let event = match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(event) => {
            validate_owner(&event)?;
            event
        }
        None => return Err(Error::NotFound(format!("Event with id={}
not found", id))),
    };

    Ok(event)
}

#[derive(CandidType, Deserialize, Serialize)]
enum Error {
    NotFound(String),
    NotAuthorized,
    AlreadyAttending,
}

fn do_insert(event: &Event) {
    STORAGE.with(|service| service.borrow_mut().insert(event.id, event.clone()));
}

fn _get_event(id: &u64) -> Option<Event> {
    STORAGE.with(|s| s.borrow().get(id))
}

fn validate_owner(event: &Event) -> Result<(), Error> {
    if event.owner != caller() {
        Err(Error::NotAuthorized)
    } else {
        Ok(())
    }
}

// need this to generate Candid
ic_cdk::export_candid!();
