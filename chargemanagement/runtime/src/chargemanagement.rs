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

        // 全体に対する情報
        AllCarsArray get(car_by_index): map u64 => T::Hash;
        AllCarsCount get(all_cars_count): u64;
        // 削除時に最後の要素と削除したい要素を入れ替えて穴が開かないようにする
        // O(1)で操作するためにインデックスを覚えておくようにする
        AllCarsIndex: map T::Hash => u64;

        // 複数の車が所持できるような構造
        OwnedCarsArray get(car_of_owner_by_index): map (T::AccountId, u64) => T::Hash;
        OwnedCarsCount get(owned_car_count): map T::AccountId => u64;
        OwnedCarsIndex: map T::Hash => u64;

        Nonce:u64;
  }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        fn deposit_event<T>() = default;

       // 車の生成
        fn create_car(origin) -> Result {
            let sender = ensure_signed(origin)?;

            // 現在の車の所持数を取得
            let owned_car_count = Self::owned_car_count(&sender);
            // 検証しつつあたらしい値を生成
            let new_owned_car_count = owned_car_count.checked_add(1).ok_or("Overflow adding a new car to account balance")?;

            // 現在のすべての車の数を取得
            let all_cars_count = Self::all_cars_count();
            // 検証しつつあたらしい値を生成
            let new_all_cars_count = all_cars_count.checked_add(1).ok_or("Overflow adding a new car to total supply")?;

            // 現在のnonceの取得、IDのハッシュ値のランダム生成
            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            // IDがすでに存在するかどうかの検証
            ensure!(!<CarOwner<T>>::exists(random_hash), "Car already exists");

            // 実際に構造体を生成
            let new_car = Car {
                id: random_hash,
                // とりあえず0に設定している
                price: <T::Balance as As<u64>>::sa(0),
            };

            <Cars<T>>::insert(random_hash, new_car);
            <CarOwner<T>>::insert(random_hash, &sender);

            // 全ての車の情報の更新
            <AllCarsArray<T>>::insert(all_cars_count, random_hash);
            <AllCarsCount<T>>::put(new_all_cars_count);
            <AllCarsIndex<T>>::insert(random_hash, all_cars_count);

            <OwnedCarsArray<T>>::insert((sender.clone(), owned_car_count), random_hash);
            <OwnedCarsCount<T>>::insert(&sender, new_owned_car_count);
            <OwnedCarsIndex<T>>::insert(random_hash, owned_car_count);

            <Nonce<T>>::mutate(|n| *n += 1);

            Self::deposit_event(RawEvent::Created(sender, random_hash));

            Ok(())
        }
    }
}