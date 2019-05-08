use support::{decl_storage, decl_module, StorageMap, StorageValue, dispatch::Result};
use system::ensure_signed;
use runtime_primitives::traits::{As, Hash};
use parity_codec::{Encode, Decode};

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Car<Hash, Balance> {
    id: Hash,
    price: Balance,
}

pub trait Trait: balances::Trait {}

decl_storage! {
    trait Store for Module<T: Trait> as ChargeManagementStorage {
          OwnedCar: map T::AccountId => Car<T::Hash, T::Balance>;
  }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn create_car(origin) -> Result {
            let sender = ensure_signed(origin)?;

            let new_car = Car {
                id: <T as system::Trait>::Hashing::hash_of(&0),
                price: <T::Balance as As<u64>>::sa(0),
            };

            <OwnedCar<T>>::insert(&sender, new_car);

            Ok(())
        }
    }
}