// This file is part of Substrate.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test utilities

use super::*;
use crate as file_bank;
use frame_support::{
    parameter_types,
    weights::Weight,
    traits::{ConstU32, EqualPrivilegeOnly, OneSessionHandler},
};
use frame_system::{EnsureRoot};
use sp_core::{H256, sr25519::Signature};
use sp_runtime::{
    testing::{Header, TestXt, UintAuthorityId},
    traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentityLookup, IdentifyAccount, Verify},
    Perbill,
};
use frame_support_test::TestRandomness;
use frame_benchmarking::account;
use pallet_cess_staking::{StashOf, Exposure, ExposureOf};
use frame_election_provider_support::{
    onchain, SequentialPhragmen, VoteWeight,
};
use sp_staking::{
    EraIndex, SessionIndex,
};
use std::cell::RefCell;
use cp_scheduler_credit::SchedulerStashAccountFinder;
/// The AccountId alias in this test module.
pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
type BlockNumber = u64;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u64;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		FileBank: file_bank::{Pallet, Call, Storage, Event<T>},
		Sminer: pallet_sminer::{Pallet, Call, Storage, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Staking: pallet_cess_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Historical: pallet_session::historical::{Pallet, Storage},
		BagsList: pallet_bags_list::{Pallet, Call, Storage, Event<T>},
		FileMap: pallet_file_map::{Pallet, Call, Storage, Event<T>},
		SchedulerCredit: pallet_scheduler_credit::{Pallet, Storage},
	}
);

parameter_types! {
	#[derive(Clone, PartialEq, Eq)]
	pub const StringLimit: u32 = 100;
	pub const OneHours: u32 = 60 * 20;
	pub const OneDay: u32 = 60 * 20 * 24;
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Test {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type MaxScheduledPerBlock = ();
    type WeightInfo = ();
    type PreimageProvider = ();
    type NoPreimagePostponement = ();
}

parameter_types! {
    pub const RewardPalletId: PalletId = PalletId(*b"sminerpt");
    pub const MultipleFines: u8 = 7;
    pub const DepositBufferPeriod: u32 = 3;
    pub const ItemLimit: u32 = 1024;
}

  impl pallet_sminer::Config for Test {
      type Currency = Balances;
      // The ubiquitous event type.
      type Event = Event;
      type PalletId = RewardPalletId;
      type SScheduler = Scheduler;
      type AScheduler = Scheduler;
      type SPalletsOrigin = OriginCaller;
      type SProposal = Call;
      type WeightInfo = ();
      type ItemLimit = ItemLimit;
      type MultipleFines = MultipleFines;
      type DepositBufferPeriod = DepositBufferPeriod;
      type CalculFailureFee = Sminer;
      type OneDayBlock = OneDay;
  }

parameter_types! {
    pub const FileMapPalletId: PalletId = PalletId(*b"filmpdpt");
}

impl pallet_file_map::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type FileMapPalletId = FileMapPalletId;
    type StringLimit = StringLimit;
    type WeightInfo = ();
	  type CreditCounter = SchedulerCredit;
}

const THRESHOLDS: [sp_npos_elections::VoteWeight; 9] =
    [10, 20, 30, 40, 50, 60, 1_000, 2_000, 10_000];

parameter_types! {
	pub static BagThresholds: &'static [sp_npos_elections::VoteWeight] = &THRESHOLDS;
	pub static MaxNominations: u32 = 16;
}

impl pallet_bags_list::Config for Test {
    type Event = Event;
    type WeightInfo = ();
    type ScoreProvider = Staking;
    type BagThresholds = BagThresholds;
    type Score = VoteWeight;
}

parameter_types! {
	pub static SessionsPerEra: SessionIndex = 3;
	pub static SlashDeferDuration: EraIndex = 0;
	pub static Period: BlockNumber = 5;
	pub static Offset: BlockNumber = 0;
}

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub other: OtherSessionHandler,
	}
}
impl pallet_session::Config for Test {
    type Event = Event;
    type ValidatorId = AccountId;
    type ValidatorIdOf = StashOf<Test>;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
    type SessionHandler = (OtherSessionHandler, );
    type Keys = SessionKeys;
    type WeightInfo = ();
}

impl pallet_session::historical::Config for Test {
    type FullIdentification = Exposure<AccountId, Balance>;
    type FullIdentificationOf = ExposureOf<Test>;
}

thread_local! {
	pub static REWARD_REMAINDER_UNBALANCED: RefCell<u128> = RefCell::new(0);
}

pub struct OnChainSeqPhragmen;

impl onchain::ExecutionConfig for OnChainSeqPhragmen {
    type System = Test;
    type Solver = SequentialPhragmen<AccountId, Perbill>;
    type DataProvider = Staking;
}

impl pallet_cess_staking::Config for Test {
    const ERAS_PER_YEAR: u64 = 8766;
    const FIRST_YEAR_VALIDATOR_REWARDS: BalanceOf<Test> = 618_000_000;
    const FIRST_YEAR_SMINER_REWARDS: BalanceOf<Test> = 309_000_000;
    const REWARD_DECREASE_RATIO: Perbill = Perbill::from_perthousand(794);
    type SminerRewardPool = ();
    type Currency = Balances;
    type UnixTime = Timestamp;
    type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
    type ElectionProvider = onchain::UnboundedExecution<OnChainSeqPhragmen>;
    type GenesisElectionProvider = Self::ElectionProvider;
    type MaxNominations = MaxNominations;
    type RewardRemainder = ();
    type Event = Event;
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = ();
    type BondingDuration = ();
    type SlashDeferDuration = ();
    type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type SessionInterface = Self;
    type EraPayout = ();
    type NextNewSession = ();
    type MaxNominatorRewardedPerValidator = ConstU32<64>;
    type OffendingValidatorsThreshold = ();
    type VoterList = BagsList;
    type MaxUnlockingChunks = ConstU32<32>;
    type BenchmarkingConfig = pallet_cess_staking::TestBenchmarkingConfig;
    type WeightInfo = ();
}

pub type Extrinsic = TestXt<Call, ()>;

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
    where
        Call: From<LocalCall>,
{
    type Extrinsic = Extrinsic;
    type OverarchingCall = Call;
}

pub struct ExtBuilder;

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {}
    }
}

pub struct OtherSessionHandler;

impl OneSessionHandler<AccountId> for OtherSessionHandler {
    type Key = UintAuthorityId;

    fn on_genesis_session<'a, I: 'a>(_: I)
        where
            I: Iterator<Item=(&'a AccountId, Self::Key)>,
            AccountId: 'a,
    {}

    fn on_new_session<'a, I: 'a>(_: bool, _: I, _: I)
        where
            I: Iterator<Item=(&'a AccountId, Self::Key)>,
            AccountId: 'a,
    {}

    fn on_disabled(_validator_index: u32) {}
}

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
    type Public = UintAuthorityId;
}

// impl ExtBuilder {
// 		#[warn(dead_code)]
//     fn build(self) -> sp_io::TestExternalities {
//         let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
//         let ext = sp_io::TestExternalities::from(storage);
//         ext
//     }
// 		#[warn(dead_code)]
//     pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
//         self.build().execute_with(test);
//     }
// }

impl frame_system::offchain::SigningTypes for Test {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
    where
        Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        _public: <Signature as Verify>::Signer,
        _account: AccountId,
        nonce: u64,
    ) -> Option<(Call, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
        Some((call, (nonce, ())))
    }
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

pub struct MockStashAccountFinder<AccountId>(PhantomData<AccountId>);

impl<AccountId: Clone> SchedulerStashAccountFinder<AccountId>
for MockStashAccountFinder<AccountId>
{
	fn find_stash_account_id(ctrl_account_id: &AccountId) -> Option<AccountId> {
		Some(ctrl_account_id.clone())
	}
}

impl pallet_scheduler_credit::Config for Test {
	type StashAccountFinder = MockStashAccountFinder<Self::AccountId>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type Balance = u64;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}



parameter_types! {
	pub const FilbakPalletId: PalletId = PalletId(*b"filebank");
}

impl Config for Test {
    type Event = Event;
    type Currency = Balances;
    type WeightInfo = ();
    type Call = Call;
    type FindAuthor = ();
		type CreditCounter = SchedulerCredit;
    type Scheduler = pallet_file_map::Pallet::<Test>;
    type MinerControl = pallet_sminer::Pallet::<Test>;
    type MyRandomness = TestRandomness<Self>;
    type FilbakPalletId = FilbakPalletId;
    type StringLimit = StringLimit;
    type OneDay = OneDay;
}

pub fn account1() -> AccountId {
    account("account1", 0, 0)
}

pub fn account2() -> AccountId {
    account("account2", 0, 0)
}

pub fn miner1() -> AccountId {
    account("miner1", 0, 0)
}

pub fn stash1() -> AccountId {
    account("stash1", 0, 0)
}

pub fn controller1() -> AccountId {
    account("controller1", 0, 0)
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (account1(), 18_000_000_000_000_000_000),
            (account2(), 1_000_000_000_000),
            (miner1(), 1_000_000_000_000),
            (stash1(), 1_000_000_000_000),
            (controller1(), 1_000_000_000_000),
        ],
    }
        .assimilate_storage(&mut t)
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1); //must set block_number, otherwise the deposit_event() don't work
    });
    ext
}

