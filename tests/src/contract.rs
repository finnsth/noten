use std::{collections::BTreeMap, path::PathBuf};
use blake2::digest::VariableOutput;
use blake2::VarBlake2b;
use blake2::digest::Update;

use casper_engine_test_support::{DEFAULT_ACCOUNT_ADDR, DEFAULT_RUN_GENESIS_REQUEST, ARG_AMOUNT, DEFAULT_PAYMENT, DeployItemBuilder, ExecuteRequestBuilder, InMemoryWasmTestBuilder};
use casper_execution_engine::core::engine_state::ExecuteRequest;
use casper_types::{account::AccountHash, ContractHash, ContractPackageHash, Key, runtime_args, RuntimeArgs, U512, U256, CLTyped, SecretKey, PublicKey, StoredValue, system::mint};
use casper_types::bytesrepr::{FromBytes, ToBytes};
use cep47::TokenId;
use maplit::btreemap;
use rand::Rng;

pub enum DeploySource {
    Code(PathBuf),
    ByContractHash {
        hash: ContractHash,
        method: String,
    },
    ByPackageHash {
        package_hash: ContractPackageHash,
        method: String,
    },
}

pub struct NotenContract {
    pub builder: InMemoryWasmTestBuilder,
    pub noten: (ContractHash, ContractPackageHash),
    pub accounts: (AccountHash, AccountHash, AccountHash, AccountHash, AccountHash, AccountHash),
}

impl NotenContract {
    pub fn base_account() -> AccountHash {
        let key = SecretKey::ed25519_from_bytes([1u8; 32]).unwrap();
        let pk = PublicKey::from(&key);
        pk.to_account_hash()
    }

    pub fn create_account() -> AccountHash {
        let key = SecretKey::ed25519_from_bytes(rand::thread_rng().gen::<[u8; 32]>()).unwrap();
        let pk = PublicKey::from(&key);
        pk.to_account_hash()
    }

    pub fn fund_account(account: &AccountHash, amount: U512) -> ExecuteRequest {
        let deploy_item = DeployItemBuilder::new()
            .with_address(*DEFAULT_ACCOUNT_ADDR)
            .with_authorization_keys(&[*DEFAULT_ACCOUNT_ADDR])
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_transfer_args(runtime_args! {
            mint::ARG_AMOUNT => amount,
            mint::ARG_TARGET => *account,
            mint::ARG_ID => <Option::<u64>>::None
        })
            .with_deploy_hash(rand::thread_rng().gen())
            .build();

        ExecuteRequestBuilder::from_deploy_item(deploy_item).build()
    }

    pub fn deploy_noten() -> Self {
        let admin = Self::create_account();
        let biff = Self::create_account();
        let tim = Self::create_account();
        let ali = Self::create_account();
        let bob = Self::create_account();
        let dan = Self::create_account();

        let mut builder = InMemoryWasmTestBuilder::default();
        let base_amount = U512::from(50_000_000_000_000_u64);

        builder.run_genesis(&DEFAULT_RUN_GENESIS_REQUEST).commit();
        builder.exec(Self::fund_account(&admin, base_amount.clone())).expect_success().commit();
        builder.exec(Self::fund_account(&biff, base_amount.clone())).expect_success().commit();
        builder.exec(Self::fund_account(&tim, base_amount.clone())).expect_success().commit();
        builder.exec(Self::fund_account(&ali, base_amount.clone())).expect_success().commit();
        builder.exec(Self::fund_account(&bob, base_amount.clone())).expect_success().commit();
        builder.exec(Self::fund_account(&dan, base_amount.clone())).expect_success().commit();

        let (nft_hash, nft_package) = Self::deploy_nft(&mut builder, &admin);

        Self {
            builder,
            noten: (nft_hash, nft_package),
            accounts: (admin, biff, tim, ali, bob, dan),
        }
    }

    pub fn deploy_nft(
        builder: &mut InMemoryWasmTestBuilder,
        admin: &AccountHash,
    ) -> (ContractHash, ContractPackageHash) {
        let token_args = runtime_args! {
            "name" => "noten",
            "symbol" => "NOT",
            "meta" => btreemap! {
                "school".to_string() => "Zuger Kantonal".to_string(),
                "kanton".to_string() => "ZG".to_string()
            },
            "contract_name" => "noten".to_string()
        };
        let nft_code = PathBuf::from("noten.wasm");
        Self::deploy(
            builder,
            admin,
            &DeploySource::Code(nft_code),
            token_args,
            true,
            None,
        );

        let contract_hash: ContractHash = Self::query(
            builder,
            Key::Account(*admin),
            &["noten_contract_hash_wrapped".to_string()],
        );
        let contract_package: ContractPackageHash = Self::query(
            builder,
            Key::Account(*admin),
            &["noten_package_hash_wrapped".to_string()],
        );
        (contract_hash, contract_package)
    }


    pub fn mint_nft<T: Into<Key>>(
        &mut self,
        recipient: T,
    ) {
        let token_meta = btreemap! {
            "origin".to_string() => "fire".to_string()
        };

        // Get the configured commissions
        let commissions = BTreeMap::<String, String>::new();

        let args = runtime_args! {
            "recipient" => recipient.into(),
            "token_meta" => token_meta,
            "token_commission" => commissions,
        };
        Self::deploy(
            &mut self.builder,
            &self.accounts.0,
            &DeploySource::ByPackageHash {
                package_hash: self.noten.1.clone(),
                method: "mint".to_string(),
            },
            args,
            true,
            None,
        );
    }

    pub fn add_teacher<T: Into<Key>>(&mut self, caller: &AccountHash, teacher: T) {
        self.call(caller, "add_teacher", runtime_args! {
            "teacher" => teacher.into(),
        }, Self::get_now_u64())
    }
    pub fn remove_teacher<T: Into<Key>>(&mut self, caller: &AccountHash, teacher: T) {
        self.call(caller, "remove_teacher", runtime_args! {
            "teacher" => teacher.into(),
        }, Self::get_now_u64())
    }
    pub fn grade<T: Into<Key>>(&mut self, caller: &AccountHash, student: T, subject: String, year: u32, grade_type: String, grade: u32) {
        self.call(caller, "grade", runtime_args! {
            "student" => student.into(),
            "subject"=> subject,
            "year" => year,
            "type" => grade_type,
            "grade" => grade,
        }, Self::get_now_u64())
    }

    pub fn query<T: FromBytes + CLTyped>(
        builder: &InMemoryWasmTestBuilder,
        base: Key,
        path: &[String],
    ) -> T {
        builder
            .query(None, base, path)
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("Wrong type in query result.")
    }

    pub fn deploy(
        builder: &mut InMemoryWasmTestBuilder,
        deployer: &AccountHash,
        source: &DeploySource,
        args: RuntimeArgs,
        success: bool,
        block_time: Option<u64>,
    ) {
        // let deploy_hash = rng.gen();
        let mut deploy_builder = DeployItemBuilder::new()
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_address(*deployer)
            .with_authorization_keys(&[*deployer])
            .with_deploy_hash(rand::thread_rng().gen());

        deploy_builder = match source {
            DeploySource::Code(path) => deploy_builder.with_session_code(path, args),
            DeploySource::ByContractHash { hash, method } => {
                deploy_builder.with_stored_session_hash(*hash, method.as_str(), args)
            }
            DeploySource::ByPackageHash {
                package_hash,
                method,
            } => deploy_builder.with_stored_versioned_contract_by_hash(
                package_hash.value(),
                None,
                method.as_str(),
                args,
            ),
        };

        let mut execute_request_builder =
            ExecuteRequestBuilder::from_deploy_item(deploy_builder.build());
        if let Some(ustamp) = block_time {
            execute_request_builder = execute_request_builder.with_block_time(ustamp)
        }
        let exec = builder.exec(execute_request_builder.build());
        if success {
            exec.expect_success()
        } else {
            exec.expect_failure()
        }
            .commit();
    }

    pub fn query_dictionary_item(
        builder: &InMemoryWasmTestBuilder,
        key: Key,
        dictionary_name: Option<String>,
        dictionary_item_key: String,
    ) -> Result<StoredValue, String> {
        let empty_path = vec![];
        let dictionary_key_bytes = dictionary_item_key.as_bytes();
        let address = match key {
            Key::Account(_) | Key::Hash(_) => {
                if let Some(name) = dictionary_name {
                    let stored_value = builder.query(None, key, &[])?;

                    let named_keys = match &stored_value {
                        StoredValue::Account(account) => account.named_keys(),
                        StoredValue::Contract(contract) => contract.named_keys(),
                        _ => {
                            return Err(
                                "Provided base key is nether an account or a contract".to_string()
                            )
                        }
                    };

                    let dictionary_uref = named_keys
                        .get(name.as_str())
                        .and_then(Key::as_uref)
                        .ok_or_else(|| "No dictionary uref was found in named keys".to_string())?;

                    Key::dictionary(*dictionary_uref, dictionary_key_bytes)
                } else {
                    return Err("No dictionary name was provided".to_string());
                }
            }
            Key::URef(uref) => Key::dictionary(uref, dictionary_key_bytes),
            Key::Dictionary(address) => Key::Dictionary(address),
            _ => return Err("Unsupported key type for a query to a dictionary item".to_string()),
        };
        builder.query(None, address, &empty_path)
    }

    pub fn get_now_u64() -> u64 {
        use std::time::SystemTime;
        match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => n.as_millis() as u64,
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        }
    }

    fn query_dictionary<T: CLTyped + FromBytes>(
        &self,
        contract: &ContractHash,
        dict_name: &str,
        key: String,
    ) -> Option<T> {
        // self.env
        //     .query_dictionary(self.nft.0.clone(), dict_name.to_string(), key)
        Self::query_dictionary_item(&self.builder,
                              Key::Hash(contract.value()),
                              Some(dict_name.to_string()),
                              key
        )
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("Wrong type in query result.")
    }

    /// Wrapper function for calling an entrypoint on the contract with the access rights of the deployer.
    pub fn call(&mut self, caller: &AccountHash, method: &str, args: RuntimeArgs, time: u64) {
        Self::deploy(
            &mut self.builder,
            caller,
            &DeploySource::ByPackageHash {
                package_hash: self.noten.1.clone(),
                method: method.to_string(),
            },
            args,
            true,
            Some(time),
        );
    }

    pub fn key_and_value_to_str<T: CLTyped + ToBytes>(key: &Key, value: &T) -> String {
        let mut hasher = VarBlake2b::new(32).unwrap();
        hasher.update(key.to_bytes().unwrap().as_slice());
        hasher.update(value.to_bytes().unwrap().as_slice());
        let mut ret = [0u8; 32];
        hasher.finalize_variable(|hash| ret.clone_from_slice(hash));
        hex::encode(ret)
    }

    pub fn get_token_by_index<T: Into<Key>>(&self, account: T, index: U256) -> Option<TokenId> {
        Self::query_dictionary_item(&self.builder,
                              Key::Hash(self.noten.0.value()),
                              Some("owned_tokens_by_index".to_string()),
                              Self::key_and_value_to_str(&account.into(), &index)
        )
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("Wrong type in query result.")
    }
}
