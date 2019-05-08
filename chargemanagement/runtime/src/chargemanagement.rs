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
        <T as system::Trait>::Hash,
        <T as balances::Trait>::Balance,
    {
        // 生成イベント
        Created(AccountId, Hash),
        // 価格設定イベント
        PriceSet(AccountId, Hash, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as ChargeManagementStorage {
        Cars get(car): map T::Hash => Car<T::Hash, T::Balance>;
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

            // 現在のnonceの取得、IDのハッシュ値のランダム生成
            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            // 実際に構造体を生成
            let new_car = Car {
                id: random_hash,
                // とりあえず0に設定している
                price: <T::Balance as As<u64>>::sa(0),
            };

            <Nonce<T>>::mutate(|n| *n += 1);

            // ストレージ変数を更新
            Self::mint(sender, random_hash, new_car)?;

            Ok(())
        }

        fn set_price(origin, car_id: T::Hash, new_price: T::Balance) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<Cars<T>>::exists(car_id), "This car does not exist");

            let owner = Self::owner_of(car_id).ok_or("No owner for this Car")?;
            ensure!(owner == sender, "You do not own this car");

            let mut car = Self::car(car_id);
            car.price = new_price;

            <Cars<T>>::insert(car_id, car);

            Self::deposit_event(RawEvent::PriceSet(sender, car_id, new_price));

            Ok(())
        }
    }
}

// プライベート関数
impl<T: Trait> Module<T> {
    // Carオブジェクトから新しい作成を作成し、すべてのストレージ変数を更新する
    fn mint(to: T::AccountId, car_id: T::Hash, new_car: Car<T::Hash, T::Balance>) -> Result {
        // IDがすでに存在するかどうかの検証
        ensure!(!<CarOwner<T>>::exists(car_id), "Car already exists");

        // 現在の車の所持数を取得
        let owned_car_count = Self::owned_car_count(&to);
        // 検証しつつあたらしい値を生成
        let new_owned_car_count = owned_car_count.checked_add(1).ok_or("Overflow adding a new car to account balance")?;

        // 現在のすべての車の数を取得
        let all_cars_count = Self::all_cars_count();
        // 検証しつつあたらしい値を生成
        let new_all_cars_count = all_cars_count.checked_add(1).ok_or("Overflow adding a new car to total supply")?;

        <Cars<T>>::insert(car_id, new_car);
        <CarOwner<T>>::insert(car_id, &to);

        // 全ての車の情報の更新
        <AllCarsArray<T>>::insert(all_cars_count, car_id);
        <AllCarsCount<T>>::put(new_all_cars_count);
        <AllCarsIndex<T>>::insert(car_id, all_cars_count);

        <OwnedCarsArray<T>>::insert((to.clone(), owned_car_count), car_id);
        <OwnedCarsCount<T>>::insert(&to, new_owned_car_count);
        <OwnedCarsIndex<T>>::insert(car_id, owned_car_count);

        Self::deposit_event(RawEvent::Created(to, car_id));

        Ok(())
    }
}