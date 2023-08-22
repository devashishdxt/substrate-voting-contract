#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod voting_contract {
    use ink::{
        prelude::{format, string::String, vec::Vec},
        storage::Mapping,
    };

    /// Defines the type of poll identifiers.
    type PollId = u64;

    /// Defines the type of choice identifiers.
    type ChoiceId = u8;

    #[ink(event)]
    /// Event emitted when a poll is created.
    pub struct PollCreated {
        #[ink(topic)]
        /// Id of the poll.
        poll_id: PollId,
        /// Description of the poll.
        description: String,
        #[ink(topic)]
        /// Account that created the poll.
        owner: AccountId,
    }

    #[ink(event)]
    /// Event emitted when a choice is added to a poll.
    pub struct ChoiceAdded {
        #[ink(topic)]
        /// Id of the poll.
        poll_id: PollId,
        #[ink(topic)]
        /// Id of the choice.
        choice_id: ChoiceId,
        /// Description of the choice.
        description: String,
    }

    #[ink(event)]
    /// Event emitted when a poll is started.
    pub struct PollStarted {
        #[ink(topic)]
        /// Id of the poll.
        poll_id: PollId,
    }

    #[ink(event)]
    /// Event emitted when a poll is ended.
    pub struct PollEnded {
        #[ink(topic)]
        /// Id of the poll.
        poll_id: PollId,
        /// Id of the winning choice (not present in case of a tie).
        winner: Option<ChoiceId>,
    }

    /// Defines the storage of the contract.
    #[ink(storage)]
    pub struct VotingContract {
        /// Stores all the polls. Maps the poll id to the poll.
        polls: Mapping<PollId, Poll>,
        /// Stores all the choices. Maps the poll id and the choice id to the choice.
        choices: Mapping<(PollId, ChoiceId), Choice>,
        /// Stores all the choice ids for a poll. Maps the poll id to a vector of choice ids.
        choice_ids: Mapping<PollId, Vec<ChoiceId>>,
        /// Stores all the votes. Maps the poll id and the choice id to the number of votes.
        vote_counts: Mapping<(PollId, ChoiceId), u64>,
        /// Used to keep track of which account has voted on a poll so that they can't vote a second time.
        voted_by: Mapping<(PollId, AccountId), bool>,
        /// Admin of the contract.
        admin: AccountId,
        /// Stores whether the contract is paused or not.
        paused: bool,
    }

    #[derive(Debug, PartialEq, Eq, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    /// A poll that can be voted on.
    pub struct Poll {
        /// Description of the poll
        description: String,
        /// Status of the poll.
        status: PollStatus,
        /// Account that created the poll.
        owner: AccountId,
        /// Winner of the poll (present if the poll has ended).
        winner: Option<ChoiceId>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    /// A choice that can be voted on.
    pub struct Choice {
        /// Description of the choice
        description: String,
    }

    #[derive(Debug, PartialEq, Eq, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    /// Status of a poll.
    pub enum PollStatus {
        /// Poll has not started yet.
        NotStarted,
        /// Poll has started.
        Started,
        /// Poll has ended.
        Ended,
    }

    /// Report generated for a poll.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PollReport {
        /// Id of the poll.
        id: PollId,
        /// Description of the poll
        description: String,
        /// Status of the poll.
        status: PollStatus,
        /// Account that created the poll.
        owner: AccountId,
        /// List of choices for the poll.
        choices: Vec<ChoiceReport>,
        /// Id of the winning choice (present if the poll has ended).
        winner: Option<ChoiceId>,
    }

    /// Report generated for a choice.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ChoiceReport {
        /// Id of the choice.
        id: ChoiceId,
        /// Description of the choice.
        description: String,
        /// Number of votes for the choice.
        vote_count: u64,
    }

    /// Errors that can occur in the voting contract.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if the poll with the given id already exist.
        PollWithIdAlreadyExists,
        /// Returned if the poll with the given id does not exist.
        PollWithIdDoesNotExist,
        /// Returned if the poll has not started yet.
        PollHasNotStarted,
        /// Returned if the poll has ended.
        PollHasEnded,
        /// Returned if the poll has already started.
        PollHasStarted,
        /// Returned if the caller is not the owner of the poll.
        OnlyOwnerCanAddChoice,
        /// Returned if the caller is not the owner of the poll.
        OnlyOwnerCanStartPoll,
        /// Returned if the owner tries to start a poll with no choices.
        CannotStartPollWithNoChoices,
        /// Returned if the caller is not the owner of the poll.
        OnlyOwnerCanEndPoll,
        /// Returned if the choice with the given id already exist.
        ChoiceWithIdAlreadyExists,
        /// Returned if the choice with the given id does not exist.
        ChoiceWithIdDoesNotExist,
        /// Returned if the caller has already voted on the poll.
        CallerAlreadyVotedOnPoll,
        /// Returned when the contract is paused by the admin.
        ContractIsPaused,
        /// Returned when the caller is not the admin.
        CallerIsNotAdmin,
        /// Returned when the contract fails to set the code hash.
        FailedToSetCodeHash(String),
    }

    impl VotingContract {
        /// Constructor for the voting contract.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self {
                polls: Mapping::new(),
                choices: Mapping::new(),
                choice_ids: Mapping::new(),
                vote_counts: Mapping::new(),
                voted_by: Mapping::new(),
                admin: Self::env().caller(),
                paused: false,
            }
        }

        #[ink(message)]
        /// Pauses the contract.
        pub fn pause(&mut self) -> Result<(), Error> {
            // Check if the caller is the admin.
            if self.env().caller() != self.admin {
                return Err(Error::CallerIsNotAdmin);
            }

            // Pause the contract.
            self.paused = true;

            Ok(())
        }

        #[ink(message)]
        /// Unpauses the contract.
        pub fn unpause(&mut self) -> Result<(), Error> {
            // Check if the caller is the admin.
            if self.env().caller() != self.admin {
                return Err(Error::CallerIsNotAdmin);
            }

            // Unpause the contract.
            self.paused = false;

            Ok(())
        }

        /// Modifies the code which is used to execute calls to this contract address (`AccountId`).
        #[ink(message)]
        pub fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), Error> {
            // Check if the caller is the admin.
            if self.env().caller() != self.admin {
                return Err(Error::CallerIsNotAdmin);
            }

            ink::env::set_code_hash(&code_hash)
                .map_err(|err| Error::FailedToSetCodeHash(format!("{:?}", err)))
        }

        #[ink(message)]
        /// Changes the admin of the contract.
        pub fn change_admin(&mut self, new_admin: AccountId) -> Result<(), Error> {
            // Check if the caller is the admin.
            if self.env().caller() != self.admin {
                return Err(Error::CallerIsNotAdmin);
            }

            // Change the admin.
            self.admin = new_admin;

            Ok(())
        }

        #[ink(message)]
        /// Creates a new poll.
        pub fn create_poll(&mut self, poll_id: PollId, description: String) -> Result<(), Error> {
            // Check if the contract is paused.
            if self.paused {
                return Err(Error::ContractIsPaused);
            }

            // Check if the poll already exists.
            if self.polls.get(&poll_id).is_some() {
                return Err(Error::PollWithIdAlreadyExists);
            }

            // Create the poll.
            let poll = Poll {
                description: description.clone(),
                status: PollStatus::NotStarted,
                owner: self.env().caller(),
                winner: None,
            };

            // Insert the poll into the storage.
            self.polls.insert(poll_id, &poll);

            // Emit the event.
            self.env().emit_event(PollCreated {
                poll_id,
                description,
                owner: self.env().caller(),
            });

            Ok(())
        }

        #[ink(message)]
        /// Adds a choice to a poll.
        pub fn add_choice(
            &mut self,
            poll_id: PollId,
            choice_id: ChoiceId,
            description: String,
        ) -> Result<(), Error> {
            // Check if the contract is paused.
            if self.paused {
                return Err(Error::ContractIsPaused);
            }

            // Get the poll and return error if it does not exist.
            let poll = self
                .polls
                .get(&poll_id)
                .ok_or(Error::PollWithIdDoesNotExist)?;

            // Check if the caller is the owner of the poll.
            if self.env().caller() != poll.owner {
                return Err(Error::OnlyOwnerCanAddChoice);
            }

            // Check if the poll has started or ended.
            match poll.status {
                PollStatus::Started => return Err(Error::PollHasStarted),
                PollStatus::Ended => return Err(Error::PollHasEnded),
                PollStatus::NotStarted => {}
            }

            // Check if the choice already exists.
            if self.choices.contains((poll_id, choice_id)) {
                return Err(Error::ChoiceWithIdAlreadyExists);
            }

            // Create the choice.
            let choice = Choice {
                description: description.clone(),
            };

            // Get the list of choice ids for the poll.
            let mut choice_id_list = self.choice_ids.get(&poll_id).unwrap_or_default();

            // Add the choice to the list of choices for the poll.
            choice_id_list.push(choice_id);

            // Insert the choice into the storage.
            self.choices.insert((poll_id, choice_id), &choice);

            // Insert the list of choice ids for the poll into the storage.
            self.choice_ids.insert(&poll_id, &choice_id_list);

            // Emit the event.
            self.env().emit_event(ChoiceAdded {
                poll_id,
                choice_id,
                description,
            });

            Ok(())
        }

        #[ink(message)]
        /// Starts a poll.
        pub fn start_poll(&mut self, poll_id: PollId) -> Result<(), Error> {
            // Check if the contract is paused.
            if self.paused {
                return Err(Error::ContractIsPaused);
            }

            // Get the poll and return error if it does not exist.
            let mut poll = self
                .polls
                .get(&poll_id)
                .ok_or(Error::PollWithIdDoesNotExist)?;

            // Check if the caller is the owner of the poll.
            if self.env().caller() != poll.owner {
                return Err(Error::OnlyOwnerCanStartPoll);
            }

            // Check if the poll has started or ended.
            match poll.status {
                PollStatus::Started => return Err(Error::PollHasStarted),
                PollStatus::Ended => return Err(Error::PollHasEnded),
                PollStatus::NotStarted => {}
            }

            self.choice_ids
                .get(&poll_id)
                .ok_or(Error::CannotStartPollWithNoChoices)?;

            // Change the status of the poll.
            poll.status = PollStatus::Started;

            // Insert the poll into the storage.
            self.polls.insert(poll_id, &poll);

            // Emit the event.
            self.env().emit_event(PollStarted { poll_id });

            Ok(())
        }

        #[ink(message)]
        /// Ends a poll.
        pub fn end_poll(&mut self, poll_id: PollId) -> Result<(), Error> {
            // Check if the contract is paused.
            if self.paused {
                return Err(Error::ContractIsPaused);
            }

            // Get the poll and return error if it does not exist.
            let mut poll = self
                .polls
                .get(&poll_id)
                .ok_or(Error::PollWithIdDoesNotExist)?;

            // Check if the caller is the owner of the poll.
            if self.env().caller() != poll.owner {
                return Err(Error::OnlyOwnerCanEndPoll);
            }

            match poll.status {
                PollStatus::Started => {}
                PollStatus::Ended => return Err(Error::PollHasEnded),
                PollStatus::NotStarted => return Err(Error::PollHasNotStarted),
            }

            // Change the status of the poll.
            poll.status = PollStatus::Ended;

            // Get the winner of the poll.
            let winner = None; // This is an intentional bug (for demonstration) to be fixed in the upgraded version.

            // Change the winner of the poll.
            poll.winner = winner;

            // Insert the poll into the storage.
            self.polls.insert(poll_id, &poll);

            // Emit the event.
            self.env().emit_event(PollEnded { poll_id, winner });

            Ok(())
        }

        #[ink(message)]
        /// Votes on a poll.
        pub fn vote(&mut self, poll_id: PollId, choice_id: ChoiceId) -> Result<(), Error> {
            // Check if the contract is paused.
            if self.paused {
                return Err(Error::ContractIsPaused);
            }

            // Get the poll and return error if it does not exist.
            let poll = self
                .polls
                .get(&poll_id)
                .ok_or(Error::PollWithIdDoesNotExist)?;

            // Check the status and return error if the poll has not started or has ended.
            match poll.status {
                PollStatus::NotStarted => return Err(Error::PollHasNotStarted),
                PollStatus::Ended => return Err(Error::PollHasEnded),
                PollStatus::Started => {}
            }

            // Check if the choice exists.
            if !self.choices.contains((poll_id, choice_id)) {
                return Err(Error::ChoiceWithIdDoesNotExist);
            }

            // Get the caller.
            let caller = self.env().caller();

            // Check if the caller has already voted on the poll.
            if self.voted_by.contains((poll_id, caller)) {
                return Err(Error::CallerAlreadyVotedOnPoll);
            }

            // Get the current vote count.
            let current_vote_count = self
                .vote_counts
                .get((poll_id, choice_id))
                .unwrap_or_default();

            // Calculate the new vote count (increment the current vote count by 1).
            let new_vote_count = current_vote_count + 1;

            // Insert the new vote count into storage.
            self.vote_counts
                .insert((poll_id, choice_id), &new_vote_count);

            // Insert the caller into storage.
            self.voted_by.insert((poll_id, caller), &true);

            Ok(())
        }

        #[ink(message)]
        /// Get all the choices for a poll.
        pub fn get_choices(&self, poll_id: PollId) -> Vec<(ChoiceId, Choice)> {
            // Get the list of choice ids for the poll.
            let choice_list = self.choice_ids.get(&poll_id).unwrap_or_default();

            // Get the choices from storage.
            choice_list
                .into_iter()
                .map(|choice_id| (choice_id, self.choices.get(&(poll_id, choice_id)).unwrap()))
                .collect()
        }

        #[ink(message)]
        /// Get the report for a poll.
        pub fn get_report(&self, poll_id: PollId) -> Result<PollReport, Error> {
            let poll = self
                .polls
                .get(&poll_id)
                .ok_or(Error::PollWithIdDoesNotExist)?;

            let choices: Vec<ChoiceReport> = self
                .choice_ids
                .get(&poll_id)
                .unwrap_or_default()
                .into_iter()
                .map(|choice_id| {
                    let choice = self.choices.get(&(poll_id, choice_id)).unwrap();

                    let vote_count = self
                        .vote_counts
                        .get(&(poll_id, choice_id))
                        .unwrap_or_default();

                    ChoiceReport {
                        id: choice_id,
                        description: choice.description,
                        vote_count,
                    }
                })
                .collect();

            let report = PollReport {
                id: poll_id,
                description: poll.description,
                status: poll.status,
                owner: poll.owner,
                choices,
                winner: poll.winner,
            };

            Ok(report)
        }
    }

    #[cfg(test)]
    mod tests {
        use ink::env::test::EmittedEvent;

        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        type Event = <VotingContract as ::ink::reflect::ContractEventBase>::Type;

        fn assert_poll_created_event(
            event: &EmittedEvent,
            expected_poll_id: PollId,
            expected_description: &str,
            expected_owner: AccountId,
        ) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("encountered invalid contract event data buffer");

            if let Event::PollCreated(PollCreated {
                poll_id,
                description,
                owner,
            }) = decoded_event
            {
                assert_eq!(poll_id, expected_poll_id);
                assert_eq!(description, expected_description);
                assert_eq!(owner, expected_owner);
            } else {
                panic!("encountered unexpected contract event kind: expected `PollCreated`")
            }
        }

        fn assert_add_choice_event(
            event: &EmittedEvent,
            expected_poll_id: PollId,
            expected_choice_id: ChoiceId,
            expected_description: &str,
        ) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("encountered invalid contract event data buffer");

            if let Event::ChoiceAdded(ChoiceAdded {
                poll_id,
                choice_id,
                description,
            }) = decoded_event
            {
                assert_eq!(poll_id, expected_poll_id);
                assert_eq!(choice_id, expected_choice_id);
                assert_eq!(description, expected_description);
            } else {
                panic!("encountered unexpected contract event kind: expected `ChoiceAdded`")
            }
        }

        fn assert_start_poll_event(event: &EmittedEvent, expected_poll_id: PollId) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("encountered invalid contract event data buffer");

            if let Event::PollStarted(PollStarted { poll_id }) = decoded_event {
                assert_eq!(poll_id, expected_poll_id);
            } else {
                panic!("encountered unexpected contract event kind: expected `PollStarted`")
            }
        }

        fn assert_end_poll_event(
            event: &EmittedEvent,
            expected_poll_id: PollId,
            expected_winner: Option<ChoiceId>,
        ) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("encountered invalid contract event data buffer");

            if let Event::PollEnded(PollEnded { poll_id, winner }) = decoded_event {
                assert_eq!(poll_id, expected_poll_id);
                assert_eq!(winner, expected_winner);
            } else {
                panic!("encountered unexpected contract event kind: expected `PollEnded`")
            }
        }

        #[ink::test]
        /// Tests that `default` constructor sets `admin` properly.
        fn test_contract_admin() {
            let voting_contract = VotingContract::default();

            assert_eq!(
                voting_contract.admin,
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().alice
            );
        }

        #[ink::test]
        /// Tests that `pause` works properly.
        fn test_contract_pause_success() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.paused, false);
            assert!(voting_contract.pause().is_ok());
            assert_eq!(voting_contract.paused, true);
        }

        #[ink::test]
        /// Tests that `pause` fails if the caller is not the admin.
        fn test_contract_pause_failure_not_admin() {
            let mut voting_contract = VotingContract::default();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().bob,
            );
            assert_eq!(voting_contract.pause(), Err(Error::CallerIsNotAdmin));
            assert_eq!(voting_contract.paused, false);
        }

        #[ink::test]
        /// Tests that `unpause` works properly.
        fn test_contract_unpause_success() {
            let mut voting_contract = VotingContract::default();

            assert!(voting_contract.pause().is_ok());

            assert!(voting_contract.unpause().is_ok());
            assert_eq!(voting_contract.paused, false);
        }

        #[ink::test]
        /// Tests that `unpause` fails if the caller is not the admin.
        fn test_contract_unpause_failure_not_admin() {
            let mut voting_contract = VotingContract::default();

            assert!(voting_contract.pause().is_ok());

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().bob,
            );
            assert_eq!(voting_contract.unpause(), Err(Error::CallerIsNotAdmin));
            assert_eq!(voting_contract.paused, true);
        }

        #[ink::test]
        /// Tests that `create_poll` works properly in success scenario.
        fn test_create_poll_success() {
            let mut voting_contract = VotingContract::default();

            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));

            // Check if the poll has been created.
            let poll = voting_contract.polls.get(&1).unwrap();
            assert_eq!(poll.description, "test".to_string());
            assert_eq!(poll.status, PollStatus::NotStarted);
            assert_eq!(
                poll.owner,
                ink::env::caller::<ink::env::DefaultEnvironment>()
            );

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_poll_created_event(
                &emitted_events[0],
                1,
                "test",
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().alice,
            );
        }

        #[ink::test]
        /// Tests that `create_poll` works properly in failure scenario (contract paused).
        fn test_create_poll_failure_contract_paused() {
            let mut voting_contract = VotingContract::default();

            assert!(voting_contract.pause().is_ok());

            assert_eq!(
                voting_contract.create_poll(1, "test".to_string()),
                Err(Error::ContractIsPaused)
            );
        }

        #[ink::test]
        /// Tests that `create_poll` works properly in failure scenario (duplicate poll id).
        fn test_create_poll_failure() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.create_poll(1, "test1".to_string()),
                Err(Error::PollWithIdAlreadyExists)
            );
        }

        #[ink::test]
        /// Tests that `add_choice` works properly in success scenario.
        fn test_add_choice_success() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.add_choice(1, 1, "test".to_string()), Ok(()));

            // Check if the choice has been added to the choice list.
            assert_eq!(voting_contract.choice_ids.get(1).unwrap().len(), 1);

            // Check if the choice has been added to choices.
            let choice = voting_contract.choices.get((1, 1)).unwrap();
            assert_eq!(choice.description, "test".to_string());

            // Add one more choice
            assert_eq!(
                voting_contract.add_choice(1, 2, "test1".to_string()),
                Ok(())
            );

            // Check if the choice has been added to the choice list.
            assert_eq!(voting_contract.choice_ids.get(1).unwrap().len(), 2);

            // Check if the choice has been added to choices.
            let choice = voting_contract.choices.get((1, 2)).unwrap();
            assert_eq!(choice.description, "test1".to_string());

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_add_choice_event(&emitted_events[1], 1, 1, "test");
            assert_add_choice_event(&emitted_events[2], 1, 2, "test1");
        }

        #[ink::test]
        /// Tests that `add_choice` works properly in failure scenario (contract paused).
        fn test_add_choice_failure_contract_paused() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));

            assert!(voting_contract.pause().is_ok());

            assert_eq!(
                voting_contract.add_choice(1, 1, "test".to_string()),
                Err(Error::ContractIsPaused)
            );
        }

        #[ink::test]
        /// Tests that `add_choice` works properly in failure scenario (duplicate choice id).
        fn test_add_choice_failure_duplicate_id() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.add_choice(1, 1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Err(Error::ChoiceWithIdAlreadyExists)
            );
        }

        #[ink::test]
        /// Tests that `add_choice` works properly in failure scenario (poll does not exist).
        fn test_add_choice_failure_poll_does_not_exist() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(
                voting_contract.add_choice(1, 1, "test".to_string()),
                Err(Error::PollWithIdDoesNotExist)
            );
        }

        #[ink::test]
        /// Tests that `add_choice` works properly in failure scenario (poll has started).
        fn test_add_choice_failure_poll_has_started() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.add_choice(1, 1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 2, "test2".to_string()),
                Err(Error::PollHasStarted)
            );
        }

        #[ink::test]
        /// Tests that `add_choice` works properly in failure scenario (poll has ended).
        fn test_add_choice_failure_poll_has_ended() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.add_choice(1, 1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(voting_contract.end_poll(1), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 2, "test2".to_string()),
                Err(Error::PollHasEnded)
            );
        }

        #[ink::test]
        /// Tests that `add_choice` works properly in failure scenario (caller is not owner).
        fn test_add_choice_failure_caller_is_not_owner() {
            let mut voting_contract = VotingContract::default();

            let default_accounts =
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.alice);
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.bob);
            assert_eq!(
                voting_contract.add_choice(1, 1, "test".to_string()),
                Err(Error::OnlyOwnerCanAddChoice)
            );
        }

        #[ink::test]
        /// Tests that `start_poll` works properly in success scenario.
        fn test_start_poll_success() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.add_choice(1, 1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.start_poll(1), Ok(()));

            // Check if the poll has been started.
            let poll = voting_contract.polls.get(1).unwrap();
            assert_eq!(poll.status, PollStatus::Started);

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_start_poll_event(&emitted_events[2], 1);
        }

        #[ink::test]
        /// Tests that `start_poll` works properly in failure scenario (contract paused).
        fn test_start_poll_failure_contract_paused() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));

            assert!(voting_contract.pause().is_ok());

            assert_eq!(voting_contract.start_poll(1), Err(Error::ContractIsPaused));
        }

        #[ink::test]
        /// Tests that `start_poll` works properly in failure scenario (poll does not exist).
        fn test_start_poll_failure_poll_does_not_exist() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(
                voting_contract.start_poll(1),
                Err(Error::PollWithIdDoesNotExist)
            );
        }

        #[ink::test]
        /// Tests that `start_poll` works properly in failure scenario (poll has started).
        fn test_start_poll_failure_poll_has_started() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.add_choice(1, 1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(voting_contract.start_poll(1), Err(Error::PollHasStarted));
        }

        #[ink::test]
        /// Tests that `start_poll` works properly in failure scenario (poll has ended).
        fn test_start_poll_failure_poll_has_ended() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.add_choice(1, 1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(voting_contract.end_poll(1), Ok(()));
            assert_eq!(voting_contract.start_poll(1), Err(Error::PollHasEnded));
        }

        #[ink::test]
        /// Tests that `start_poll` works properly in failure scenario (caller is not owner).
        fn test_start_poll_failure_caller_is_not_owner() {
            let mut voting_contract = VotingContract::default();

            let default_accounts =
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.alice);
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.bob);
            assert_eq!(
                voting_contract.start_poll(1),
                Err(Error::OnlyOwnerCanStartPoll)
            );
        }

        #[ink::test]
        /// Tests that `end_poll` works properly in success scenario.
        fn test_end_poll_success() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(
                voting_contract.add_choice(1, 2, "test2".to_string()),
                Ok(())
            );
            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(voting_contract.end_poll(1), Ok(()));

            // Check if the poll has been ended.
            let poll = voting_contract.polls.get(1).unwrap();
            assert_eq!(poll.status, PollStatus::Ended);

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_end_poll_event(&emitted_events[4], 1, None);
        }

        #[ink::test]
        /// Tests that `end_poll` works properly in failure scenario (contract paused).
        fn test_end_poll_failure_contract_paused() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.add_choice(1, 1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.start_poll(1), Ok(()));

            assert!(voting_contract.pause().is_ok());

            assert_eq!(voting_contract.end_poll(1), Err(Error::ContractIsPaused));
        }

        #[ink::test]
        /// Tests that `end_poll` works properly in failure scenario (poll does not exist).
        fn test_end_poll_failure_poll_does_not_exist() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(
                voting_contract.end_poll(1),
                Err(Error::PollWithIdDoesNotExist)
            );
        }

        #[ink::test]
        /// Tests that `end_poll` works properly in failure scenario (poll has ended).
        fn test_end_poll_failure_poll_has_ended() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.add_choice(1, 1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(voting_contract.end_poll(1), Ok(()));
            assert_eq!(voting_contract.end_poll(1), Err(Error::PollHasEnded));
        }

        #[ink::test]
        /// Tests that `end_poll` works properly in failure scenario (poll has not started).
        fn test_end_poll_failure_poll_has_not_started() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.end_poll(1), Err(Error::PollHasNotStarted));
        }

        #[ink::test]
        /// Tests that `end_poll` works properly in failure scenario (caller is not owner).
        fn test_end_poll_failure_caller_is_not_owner() {
            let mut voting_contract = VotingContract::default();

            let default_accounts =
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.alice);
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.add_choice(1, 1, "test".to_string()), Ok(()));
            assert_eq!(voting_contract.start_poll(1), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.bob);
            assert_eq!(voting_contract.end_poll(1), Err(Error::OnlyOwnerCanEndPoll));
        }

        #[ink::test]
        /// Tests that `vote` works properly in success scenario.
        fn test_vote_success() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(voting_contract.vote(1, 1), Ok(()));

            // Check if the vote has been registered.
            let vote_count = voting_contract.vote_counts.get((1, 1)).unwrap();
            assert_eq!(vote_count, 1);
            assert!(voting_contract
                .voted_by
                .contains(&(1, ink::env::caller::<ink::env::DefaultEnvironment>())));
        }

        #[ink::test]
        /// Tests that `vote` works properly in failure scenario (contract paused).
        fn test_vote_failure_contract_paused() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(voting_contract.start_poll(1), Ok(()));

            assert!(voting_contract.pause().is_ok());

            assert_eq!(voting_contract.vote(1, 1), Err(Error::ContractIsPaused));
        }

        #[ink::test]
        /// Tests that `vote` works properly in failure scenario (poll does not exist).
        fn test_vote_failure_poll_does_not_exist() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(
                voting_contract.vote(1, 1),
                Err(Error::PollWithIdDoesNotExist)
            );
        }

        #[ink::test]
        /// Tests that `vote` works properly in failure scenario (poll has ended).
        fn test_vote_failure_poll_has_ended() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(voting_contract.end_poll(1), Ok(()));
            assert_eq!(voting_contract.vote(1, 1), Err(Error::PollHasEnded));
        }

        #[ink::test]
        /// Tests that `vote` works properly in failure scenario (poll has not started).
        fn test_vote_failure_poll_has_not_started() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(voting_contract.vote(1, 1), Err(Error::PollHasNotStarted));
        }

        #[ink::test]
        /// Tests that `vote` works properly in failure scenario (choice does not exist).
        fn test_vote_failure_choice_does_not_exist() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(
                voting_contract.vote(1, 2),
                Err(Error::ChoiceWithIdDoesNotExist)
            );
        }

        #[ink::test]
        /// Tests that `vote` works properly in failure scenario (caller has already voted).
        fn test_vote_failure_caller_has_already_voted() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(voting_contract.vote(1, 1), Ok(()));
            assert_eq!(
                voting_contract.vote(1, 1),
                Err(Error::CallerAlreadyVotedOnPoll)
            );
        }

        #[ink::test]
        /// Tests that `get_report` returns the correct report (poll has not started).
        fn test_get_report_poll_has_not_started() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );

            let report = voting_contract.get_report(1).unwrap();

            assert_eq!(report.id, 1);
            assert_eq!(report.description, "test".to_string());
            assert_eq!(report.status, PollStatus::NotStarted);

            assert_eq!(report.choices.len(), 1);

            assert_eq!(report.choices[0].id, 1);
            assert_eq!(report.choices[0].description, "test1".to_string());
            assert_eq!(report.choices[0].vote_count, 0);

            assert_eq!(report.winner, None);
        }

        #[ink::test]
        /// Tests that `get_report` returns the correct report (poll has started).
        fn test_get_report_poll_has_started() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );

            assert_eq!(voting_contract.start_poll(1), Ok(()));

            let report = voting_contract.get_report(1).unwrap();

            assert_eq!(report.id, 1);
            assert_eq!(report.description, "test".to_string());
            assert_eq!(report.status, PollStatus::Started);

            assert_eq!(report.choices.len(), 1);

            assert_eq!(report.choices[0].id, 1);
            assert_eq!(report.choices[0].description, "test1".to_string());
            assert_eq!(report.choices[0].vote_count, 0);

            assert_eq!(report.winner, None);
        }

        #[ink::test]
        /// Tests that `get_report` returns the correct report (poll has started with votes).
        fn test_get_report_poll_has_started_with_votes() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(
                voting_contract.add_choice(1, 2, "test2".to_string()),
                Ok(())
            );

            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(voting_contract.vote(1, 1), Ok(()));

            let report = voting_contract.get_report(1).unwrap();

            assert_eq!(report.id, 1);
            assert_eq!(report.description, "test".to_string());
            assert_eq!(report.status, PollStatus::Started);

            assert_eq!(report.choices.len(), 2);

            assert_eq!(report.choices[0].id, 1);
            assert_eq!(report.choices[0].description, "test1".to_string());
            assert_eq!(report.choices[0].vote_count, 1);

            assert_eq!(report.choices[1].id, 2);
            assert_eq!(report.choices[1].description, "test2".to_string());
            assert_eq!(report.choices[1].vote_count, 0);

            assert_eq!(report.winner, None);
        }

        #[ink::test]
        /// Tests that `get_report` returns the correct report (poll has ended with votes).
        fn test_get_report_poll_has_ended_with_votes() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(
                voting_contract.add_choice(1, 2, "test2".to_string()),
                Ok(())
            );

            assert_eq!(voting_contract.start_poll(1), Ok(()));
            assert_eq!(voting_contract.vote(1, 2), Ok(()));
            assert_eq!(voting_contract.end_poll(1), Ok(()));

            let report = voting_contract.get_report(1).unwrap();

            assert_eq!(report.id, 1);
            assert_eq!(report.description, "test".to_string());
            assert_eq!(report.status, PollStatus::Ended);

            assert_eq!(report.choices.len(), 2);

            assert_eq!(report.choices[0].id, 1);
            assert_eq!(report.choices[0].description, "test1".to_string());
            assert_eq!(report.choices[0].vote_count, 0);

            assert_eq!(report.choices[1].id, 2);
            assert_eq!(report.choices[1].description, "test2".to_string());
            assert_eq!(report.choices[1].vote_count, 1);

            assert_eq!(report.winner, None);
        }

        #[ink::test]
        /// Tests that `get_choices` returns the correct choices.
        fn test_get_choices() {
            let mut voting_contract = VotingContract::default();
            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(
                voting_contract.add_choice(1, 2, "test2".to_string()),
                Ok(())
            );
            assert_eq!(
                voting_contract.add_choice(1, 3, "test3".to_string()),
                Ok(())
            );

            let choices = voting_contract.get_choices(1);
            assert_eq!(choices.len(), 3);
            assert_eq!(choices[0].0, 1);
            assert_eq!(choices[1].0, 2);
            assert_eq!(choices[2].0, 3);

            assert_eq!(choices[0].1.description, "test1".to_string());
            assert_eq!(choices[1].1.description, "test2".to_string());
            assert_eq!(choices[2].1.description, "test3".to_string());
        }

        #[ink::test]
        /// Tests full flow of the contract
        fn test_full_flow() {
            let mut voting_contract = VotingContract::default();

            let default_accounts =
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.alice);

            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(
                voting_contract.add_choice(1, 2, "test2".to_string()),
                Ok(())
            );
            assert_eq!(
                voting_contract.add_choice(1, 3, "test3".to_string()),
                Ok(())
            );

            assert_eq!(voting_contract.start_poll(1), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.bob);
            assert_eq!(voting_contract.vote(1, 1), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.eve);
            assert_eq!(voting_contract.vote(1, 2), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.alice);
            assert_eq!(voting_contract.vote(1, 3), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.django);
            assert_eq!(voting_contract.vote(1, 2), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.frank);
            assert_eq!(voting_contract.vote(1, 2), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.alice);
            assert_eq!(voting_contract.end_poll(1), Ok(()));

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_poll_created_event(&emitted_events[0], 1, "test", default_accounts.alice);
            assert_add_choice_event(&emitted_events[1], 1, 1, "test1");
            assert_add_choice_event(&emitted_events[2], 1, 2, "test2");
            assert_add_choice_event(&emitted_events[3], 1, 3, "test3");
            assert_start_poll_event(&emitted_events[4], 1);
            assert_end_poll_event(&emitted_events[5], 1, None);

            let report = voting_contract.get_report(1).unwrap();

            assert_eq!(report.id, 1);
            assert_eq!(report.description, "test".to_string());
            assert_eq!(report.status, PollStatus::Ended);
            assert_eq!(report.winner, None);
            assert_eq!(report.owner, default_accounts.alice);

            assert_eq!(report.choices.len(), 3);

            assert_eq!(report.choices[0].id, 1);
            assert_eq!(report.choices[0].description, "test1".to_string());
            assert_eq!(report.choices[0].vote_count, 1);

            assert_eq!(report.choices[1].id, 2);
            assert_eq!(report.choices[1].description, "test2".to_string());
            assert_eq!(report.choices[1].vote_count, 3);

            assert_eq!(report.choices[2].id, 3);
            assert_eq!(report.choices[2].description, "test3".to_string());
            assert_eq!(report.choices[2].vote_count, 1);
        }

        #[ink::test]
        /// Tests full flow of the contract (with tie)
        fn test_full_flow_with_tie() {
            let mut voting_contract = VotingContract::default();

            let default_accounts =
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.alice);

            assert_eq!(voting_contract.create_poll(1, "test".to_string()), Ok(()));
            assert_eq!(
                voting_contract.add_choice(1, 1, "test1".to_string()),
                Ok(())
            );
            assert_eq!(
                voting_contract.add_choice(1, 2, "test2".to_string()),
                Ok(())
            );
            assert_eq!(
                voting_contract.add_choice(1, 3, "test3".to_string()),
                Ok(())
            );

            assert_eq!(voting_contract.start_poll(1), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.bob);
            assert_eq!(voting_contract.vote(1, 1), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.eve);
            assert_eq!(voting_contract.vote(1, 2), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.alice);
            assert_eq!(voting_contract.vote(1, 3), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.django);
            assert_eq!(voting_contract.vote(1, 2), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.frank);
            assert_eq!(voting_contract.vote(1, 3), Ok(()));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(default_accounts.alice);
            assert_eq!(voting_contract.end_poll(1), Ok(()));

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert_poll_created_event(&emitted_events[0], 1, "test", default_accounts.alice);
            assert_add_choice_event(&emitted_events[1], 1, 1, "test1");
            assert_add_choice_event(&emitted_events[2], 1, 2, "test2");
            assert_add_choice_event(&emitted_events[3], 1, 3, "test3");
            assert_start_poll_event(&emitted_events[4], 1);
            assert_end_poll_event(&emitted_events[5], 1, None);

            let report = voting_contract.get_report(1).unwrap();

            assert_eq!(report.id, 1);
            assert_eq!(report.description, "test".to_string());
            assert_eq!(report.status, PollStatus::Ended);
            assert_eq!(report.winner, None);
            assert_eq!(report.owner, default_accounts.alice);

            assert_eq!(report.choices.len(), 3);

            assert_eq!(report.choices[0].id, 1);
            assert_eq!(report.choices[0].description, "test1".to_string());
            assert_eq!(report.choices[0].vote_count, 1);

            assert_eq!(report.choices[1].id, 2);
            assert_eq!(report.choices[1].description, "test2".to_string());
            assert_eq!(report.choices[1].vote_count, 2);

            assert_eq!(report.choices[2].id, 3);
            assert_eq!(report.choices[2].description, "test3".to_string());
            assert_eq!(report.choices[2].vote_count, 2);
        }
    }
}
