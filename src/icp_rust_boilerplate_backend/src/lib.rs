#[macro_use]
    extern crate serde;
    use candid::{Decode, Encode};
    use ic_cdk::api::time;
    use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
    use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
    use std::{borrow::Cow, cell::RefCell};
    use ic_cdk::caller;

    type Memory = VirtualMemory<DefaultMemoryImpl>;
    type IdCell = Cell<u64, Memory>;


    #[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
    struct Event {
        id: u64,
        eventDescription: String,
        owner: String,
        eventTitle: String,
        eventLocation : String,
        eventCardImgUrl : String,
        attendees : Vec<String>,
        created_at: u64,
        updated_at: Option<u64>,
    }

     // a trait that must be implemented for a struct that is stored in a stable struct
     impl Storable for Event {
        fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
            Cow::Owned(Encode!(self).unwrap())
        }
    
        fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
            Decode!(bytes.as_ref(), Self).unwrap()
        }
    }
    
    // another trait that must be implemented for a struct that is stored in a stable struct
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


    #[derive(candid::CandidType, Serialize, Deserialize, Default)]
    struct EventPayload {
        eventDescription: String,
        eventTitle: String,
        eventLocation : String,
        eventCardImgUrl : String,
    }


    #[ic_cdk::query]
    fn get_event(id: u64) -> Result<Event, Error> {
        match _get_event(&id) {
            Some(message) => Ok(message),
            None => Err(Error::NotFound {
                msg: format!("Event with id={} not found", id),
            }),
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
            eventDescription: payload.eventDescription,
            owner: caller().to_string(),
            eventTitle: payload.eventTitle,
            eventLocation : payload.eventLocation,
            eventCardImgUrl : payload.eventCardImgUrl,
            attendees : Vec::new(),
            created_at: time(),
            updated_at: None,
        };
        do_insert(&event);
        Some(event)
    }


    #[ic_cdk::update]
    fn update_event(id: u64, payload: EventPayload) -> Result<Event, Error> {
        match STORAGE.with(|service| service.borrow().get(&id)) {
            Some(mut event) => {
                event.eventDescription = payload.eventDescription;
                event.eventTitle = payload.eventTitle;
                event.eventLocation  = payload.eventLocation;
                event.eventCardImgUrl  = payload.eventCardImgUrl;
                event.updated_at = Some(time());
                do_insert(&event);
                Ok(event)
            }
            None => Err(Error::NotFound {
                msg: format!(
                    "couldn't update an event with id={}. event not found",
                    id
                ),
            }),
        }
    }


    #[ic_cdk::update]
    fn attend_event(id: u64) -> Result<Event, Error> {
        match STORAGE.with(|service| service.borrow().get(&id)) {
            Some(mut event) => {
               let  user = caller().to_string();

                let mut attendees: Vec<String> = event.attendees;
        
                attendees.push(user);
                
                event.attendees = attendees;

                do_insert(&event);
                Ok(event)
            }
            None => Err(Error::NotFound {
                msg: format!(
                    "couldn't update an event with id={}. event not found",
                    id
                ),
            }),
        }
    }


    

    #[ic_cdk::update]
    fn delete_event(id: u64) -> Result<Event, Error> {
        match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
            Some(message) => Ok(message),
            None => Err(Error::NotFound {
                msg: format!(
                    "couldn't delete event with id={}. event not found.",
                    id
                ),
            }),
        }
    }



     // Helper method to insert an event.
     fn do_insert(event: &Event) {
        STORAGE.with(|service| service.borrow_mut().insert(event.id, event.clone()));
    }

    // Helper method to retrieve an event by it's id 
    fn _get_event(id: &u64) -> Option<Event> {
        STORAGE.with(|s| s.borrow().get(id))
    }
    
    #[derive(candid::CandidType, Deserialize, Serialize)]
    enum Error {
        NotFound { msg: String },
    }



    // need this to generate candid
    ic_cdk::export_candid!();


    