use support::{decl_storage, decl_module, StorageValue, StorageMap, dispatch::Result, ensure, decl_event};
use system::ensure_signed;
use runtime_primitives::traits::{As, Hash};
use parity_codec::{Encode, Decode};

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Car<Hash, Balance> {
    id: Hash,
    price: Balance,
}

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash
    {
        Created(AccountId, Hash),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as ChargeManagementStorage {
        Cars get(cars): map T::Hash => Car<T::Hash, T::Balance>;
        CarOwner get(owner_of): map T::Hash => Option<T::AccountId>;
        OwnedCar get(car_ofowner): map T::AccountId => T::Hash;

        Nonce:u64;
  }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        fn deposit_event<T>() = default;

        fn create_car(origin) -> Result {
            let sender = ensure_signed(origin)?;

            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            ensure!(!<CarOwner<T>>::exists(random_hash), "Car already exists");

            let new_car = Car {
                id: <T as system::Trait>::Hashing::hash_of(&0),
                price: <T::Balance as As<u64>>::sa(0),
            };

            <Cars<T>>::insert(random_hash, new_car);
            <CarOwner<T>>::insert(random_hash, &sender);
            <OwnedCar<T>>::insert(&sender, random_hash);

            <Nonce<T>>::mutate(|n| *n += 1);

            Ok(())
        }
    }
}