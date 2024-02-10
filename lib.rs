#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod ronin_mission5_user {

    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;
    use scale::{Decode, Encode};

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum CrudError {
        MessageAlreadyCreatedBySender,
        MessageTooShort,
        MessageIsIdentical,
        AnyMessageFound,
        Unauthorized,
    }

    /* Use a custom struct Message instead as (AccountId, String) */
    #[derive(Debug, PartialEq, Eq, Encode, Decode, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Message {
        sender: AccountId,
        message: String,
        created_at: Timestamp,
        updated_at: Timestamp,
        deleted_at: Option<Timestamp>,
    }

    impl Message {
        pub fn new(sender: AccountId, message: String, created_at: Timestamp) -> Self {
            Self {
                sender,
                message,
                created_at,
                updated_at: created_at, // Set updated_at to created_at (first message is created at the same time as updated_at)
                deleted_at: None,
            }
        }

        pub fn delete(&mut self, deleted_at: Timestamp) {
            self.deleted_at = Some(deleted_at);
        }

        pub fn update(&mut self, message: String, updated_at: Timestamp) {
            self.message = message;
            self.updated_at = updated_at;
        }
    }

    #[ink(storage)]
    pub struct CrudContract {
        messages: Vec<Message>,
        creator: AccountId, // creator added: to check if caller is contract owner
        // senders removed: replace by a public get_senders function
    }

    impl CrudContract {

        /* Constructor */
        #[ink(constructor)]
        pub fn new() -> Self {
            let creator: AccountId = Self::env().caller();

            let mut messages: Vec<Message> = Vec::<Message>::new();

            let init_message: String = String::from("I created my ULTIMATE CRUD contract for Ronin Club");

            messages.push(Message::new(creator, init_message, Self::env().block_timestamp()));

            Self { messages, creator }
        }


        /* Public function - Create a message
        *  Check if message has already been created by sender
        *  Check if message has a minimal length of 10
        */
        #[ink(message)]
        pub fn create_message(&mut self, message: String) -> Result<(), CrudError> {
            let caller: AccountId = self.env().caller();

            /* Verify if message has already been created by sender */
            self.can_create_message(caller)?;

            /* Verify if message has a minimal length of 10 */
            self.is_message_too_short(&message)?;

            // insert message
            self.messages.push(Message::new(caller, message, Self::env().block_timestamp()));

            Ok(())
        }

        /* Public function - Return a message from sender
        *  Check sender has not deleted message in storage
        */
        #[ink(message)]
        pub fn read_message_from(&mut self, caller: AccountId) -> Result<String, CrudError> {

            let caller_mesage: Option<Message> = self.get_caller_message(caller);

            /* Verify if sender has not deleted message in storage */
            if caller_mesage.is_some() {
                return Ok(caller_mesage.unwrap().message);
            } else {
                return Err(CrudError::AnyMessageFound);
            }
        }

        /* Public function - Read all messages
        *  Check if caller is contract creator
        */
        #[ink(message)]
        pub fn read_all_messages(&mut self) -> Result<Vec<Message>, CrudError> {

            /* Verify if caller is contract owner */
            self.is_authorized(self.env().caller())?;

            let all_messages: Vec<Message> = self.get_all_messages_from_storage();

            /* Verify if messages is empty */
            if all_messages.is_empty() {
                return Err(CrudError::AnyMessageFound);
            }

            return Ok(all_messages);
        }

        /* Public function - Update caller message
        *  Check if message has already been created by sender and not deleted
        *  Check if message has a minimal length of 10
        *  Check if last message is identical
        */
        #[ink(message)]
        pub fn update_message(&mut self, message: String) -> Result<(), CrudError> {
            let caller: AccountId = self.env().caller();

            /* Verify if message has already been created by sender */
            self.can_edit_message(caller)?;

            /* Verify if message has a minimal length of 10 */
            self.is_message_too_short(&message)?;

            /* Verify if last message is identical */
            if self.get_caller_message(caller).unwrap().message == message {
                return Err(CrudError::MessageIsIdentical);
            }

            // Update message using struct method
            self.get_caller_mut_message(caller).unwrap().update(message, Self::env().block_timestamp());

            Ok(())
        }

        /* Public function - Delete caller message
        *  Check if message has already been created by sender and not deleted
        */
        #[ink(message)]
        pub fn delete_message(&mut self) -> Result<(), CrudError> {
            let caller: AccountId = self.env().caller();

            /* Verify if message has already been created by sender */
            self.can_edit_message(caller)?;

            /* Delete message */
            self.get_caller_mut_message(caller).unwrap().delete(Self::env().block_timestamp());

            Ok(())
        }

        /* Public function - Get senders
        * Get senders from all readable messages
        * Maxi Bonus feature
        */
        #[ink(message)]
        pub fn get_senders(&self) -> Vec<AccountId> {
            let mut senders: Vec<AccountId> = Vec::<AccountId>::new();

            for m in self.messages.clone() {
                if m.deleted_at.is_none() {
                    senders.push(m.sender);
                }
            }

            senders.sort();
            senders.dedup();

            return senders;
        }

        // Private function to check if caller is authorized
        fn is_authorized(&self, caller: AccountId) -> Result<(), CrudError> {
            if caller != self.creator {
                return Err(CrudError::Unauthorized);
            }
            Ok(())
        }

        // Private function to return Result CrudError if message is too short
        fn is_message_too_short(&self, message: &String) -> Result<(), CrudError> {
            if message.len() < 10 {
                return Err(CrudError::MessageTooShort);
            }
            Ok(())
        }

        // Private function to return Result CrudError if caller has message can be updated
        fn can_edit_message(&self, caller: AccountId) -> Result<(), CrudError> {
            if self.get_caller_message(caller).is_some() {
                return Ok(())
            } else {
                return Err(CrudError::AnyMessageFound);
            }
        }

        // Private function to return Result CrudError if caller can create message
        fn can_create_message(&self, caller: AccountId) -> Result<(), CrudError> {
            if self.get_caller_message(caller).is_none() {
                return Ok(())
            } else {
                return Err(CrudError::MessageAlreadyCreatedBySender);
            }
        }

        // Private fonction to get all messages from storage
        fn get_all_messages_from_storage(&self) -> Vec<Message> {
            let mut all_messages: Vec<Message> = Vec::<Message>::new();

            for m in self.messages.clone() {
                all_messages.push(m);
            }

            all_messages.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            return all_messages;
        }

        /* Private function to get caller mutable message 
        * Latest, not deleted message
        */
        fn get_caller_mut_message(&mut self, caller: AccountId) -> Option<&mut Message> {
            return self.messages.iter_mut().find(|m: &&mut Message| m.sender == caller && m.deleted_at.is_none());
        }

        /* Private function to get caller message 
        * Latest, not deleted message
        */
        fn get_caller_message(&self, caller: AccountId) -> Option<Message> {
            return self.messages.iter().find(|m: &&Message| m.sender == caller && m.deleted_at.is_none()).cloned();
        }

    }
}
