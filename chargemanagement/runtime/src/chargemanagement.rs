use support::{decl_storage, decl_module, StorageValue, StorageMap, dispatch::Result, ensure, decl_event, traits::Currency};
use system::ensure_signed;
use runtime_primitives::traits::{As, Hash, Zero};
use parity_codec::{Encode, Decode};
use rstd::cmp;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Car<Hash, Balance> {
    id: Hash,
    price: Balance,
    stored_fee: Balance,
}

// {
//   "Car": {
//     "id": "Hash",
//     "price": "Balance",
//     "stored_fee": "Balance"
//   }
// }

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
        Transferred(AccountId, AccountId, Hash),
        Bought(AccountId, AccountId, Hash, Balance),
        FeePaid(Hash, Balance),
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
                stored_fee: <T::Balance as As<u64>>::sa(0),
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

        fn transfer(origin, to: T::AccountId, car_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;

            let owner = Self::owner_of(car_id).ok_or("No owner for this car")?;
            ensure!(owner == sender, "You do not own this car");

            Self::transfer_from(sender, to, car_id)?;

            Ok(())
        }

        fn buy_car(origin, car_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<Cars<T>>::exists(car_id), "This car does not exist");

            let owner = Self::owner_of(car_id).ok_or("No owner for this car")?;
            ensure!(owner != sender, "You can't buy your own car");

            let mut car = Self::car(car_id);

            let car_price = car.price;
            ensure!(!car_price.is_zero(), "The car you want to buy is not for sale");

            <balances::Module<T> as Currency<_>>::transfer(&sender, &owner, car_price)?;

            Self::transfer_from(owner.clone(), sender.clone(), car_id)
                .expect("`owner` is shown to own the car; \
                `owner` must have greater than 0 car, so transfer cannot cause underflow; \
                `all_car_count` shares the same type as `owned_car_count` \
                and minting ensure there won't ever be more than `max()` cars, \
                which means transfer cannot cause an overflow; \
                qed");

            car.price = <T::Balance as As<u64>>::sa(0);
            <Cars<T>>::insert(car_id, car);

            Self::deposit_event(RawEvent::Bought(sender, owner, car_id, car_price));

            Ok(())
        }

        /// 指定したIDの車に対して料金の支払いを行います。
        fn pay_fee_of(origin, car_id: T::Hash, amount: T::Balance) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<Cars<T>>::exists(car_id), "This car does not exist");

            let mut car = Self::car(car_id);

            let old_stored_fee = car.stored_fee;
            car.stored_fee = car.stored_fee + amount;
            
            <Cars<T>>::insert(car_id, car);
            Self::deposit_event(RawEvent::FeePaid(car_id, amount));

            // TODO: 現状ではただ単にstore内のBalanceが増えているだけなので実際に支払いを行えるようにする必要がある。

            Ok(())
        }
    }
}

// プライベート関数
impl<T: Trait> Module<T> {
    // Carオブジェクトから新しい車を作成し、すべてのストレージ変数を更新する
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

    // 車を送る
    fn transfer_from(from: T::AccountId, to: T::AccountId, car_id: T::Hash) -> Result {
        let owner = Self::owner_of(car_id).ok_or("No owner for this car")?;

        ensure!(owner == from, "'from' account does not own this car");

        let owned_car_count_from = Self::owned_car_count(&from);
        let owned_car_count_to = Self::owned_car_count(&to);

        let new_owned_car_count_to = owned_car_count_to.checked_add(1)
            .ok_or("Transfer causes overflow of 'to' car balance")?;

        let new_owned_car_count_from = owned_car_count_from.checked_sub(1)
            .ok_or("Transfer causes underflow of 'from' car balance")?;

        // "Swap and pop"
        let car_index = <OwnedCarsIndex<T>>::get(car_id);
        if car_index != new_owned_car_count_from {
            let last_car_id = <OwnedCarsArray<T>>::get((from.clone(), new_owned_car_count_from));
            <OwnedCarsArray<T>>::insert((from.clone(), car_index), last_car_id);
            <OwnedCarsIndex<T>>::insert(last_car_id, car_index);
        }

        <CarOwner<T>>::insert(&car_id, &to);
        <OwnedCarsIndex<T>>::insert(car_id, owned_car_count_to);

        <OwnedCarsArray<T>>::remove((from.clone(), new_owned_car_count_from));
        <OwnedCarsArray<T>>::insert((to.clone(), owned_car_count_to), car_id);

        <OwnedCarsCount<T>>::insert(&from, new_owned_car_count_from);
        <OwnedCarsCount<T>>::insert(&to, new_owned_car_count_to);

        Self::deposit_event(RawEvent::Transferred(from, to, car_id));

        Ok(())
    }
}