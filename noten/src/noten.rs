#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::{format, vec};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::AtomicU32;
use casper_contract::contract_api::{runtime, storage};
use casper_contract::unwrap_or_revert::UnwrapOrRevert;
use casper_types::{ApiError, CLType, CLTyped, CLValue, ContractPackageHash, EntryPoint, EntryPointAccess, EntryPoints, EntryPointType, Group, Key, Parameter, runtime_args, RuntimeArgs, URef};
use cep47::{CEP47, Error, Meta, TokenId};
use cep47::contract_utils::{AdminControl, ContractContext, OnChainContractStorage};

#[derive(Default)]
struct NotenContract(OnChainContractStorage);


impl ContractContext<OnChainContractStorage> for NotenContract {
    fn storage(&self) -> &OnChainContractStorage {
        &self.0
    }
}

impl CEP47<OnChainContractStorage> for NotenContract {}
impl AdminControl<OnChainContractStorage> for NotenContract {}

impl NotenContract {
    fn constructor(&mut self, name: String, symbol: String, meta: Meta) {
        CEP47::init(self, name, symbol, meta);
        AdminControl::init(self);
    }
    fn grade(&mut self, student: Key, subject: String, year: u32, grade_type: String, grade: u32) -> Result<Vec<TokenId>, Error> {
        //Check if the person who is calling this code is a teacher
        let teacher = runtime::get_caller();
        let admin = self.is_admin(Key::from(teacher));
        if !admin {
            runtime::revert(ApiError::User(20));
        }
        let mut meta = Meta::new();
        meta.insert("subject".to_string(), subject);
        meta.insert("year" .to_string(), year.to_string());
        meta.insert("grade_type".to_string(), grade_type);
        meta.insert("grade".to_string(), grade.to_string());
        let mut list_of_meta = Vec::<Meta>::new();
        list_of_meta.push(meta);

        self.mint(student, Option::None, list_of_meta)


    }
    fn update_grade(&self, student: Key, grade: u32, token_id: TokenId) {}
    fn remove_teacher(&mut self, teacher: Key) {
        self.disable_admin(teacher)
    }

    fn add_teacher(&mut self, teacher: Key) {
        self.add_admin(teacher)
    }
}

#[no_mangle]
fn constructor() {
    let name = runtime::get_named_arg::<String>("name");
    let symbol = runtime::get_named_arg::<String>("symbol");
    let meta = runtime::get_named_arg::<Meta>("meta");
    NotenContract::default().constructor(name, symbol, meta);
    NotenContract::default().add_admin_without_checked(Key::Account(runtime::get_caller()));
}

#[no_mangle]
fn name() {
    let ret = NotenContract::default().name();
    runtime::ret(CLValue::from_t(ret).unwrap_or_revert());
}

#[no_mangle]
fn symbol() {
    let ret = NotenContract::default().symbol();
    runtime::ret(CLValue::from_t(ret).unwrap_or_revert());
}

#[no_mangle]
fn meta() {
    let ret = NotenContract::default().meta();
    runtime::ret(CLValue::from_t(ret).unwrap_or_revert());
}

/*
teachers can give grades to students
 */
#[no_mangle]
pub extern "C" fn grade() {
    let student:Key  = runtime::get_named_arg("student");
    let subject:String = runtime::get_named_arg("subject");
    let year:u32 = runtime::get_named_arg("year");
    let grade_type:String = runtime::get_named_arg("type");
    let grade:u32 = runtime::get_named_arg("grade");
    NotenContract::default().grade(student, subject, year, grade_type, grade);
}
/*
teachers can update/change the grade
 */
#[no_mangle]
pub extern "C" fn update_grade() {
    let student: Key = runtime::get_named_arg("student");
    let grade: u32 = runtime::get_named_arg("grade");
    let token_id:TokenId = runtime::get_named_arg("token_id");
    NotenContract::default().update_grade(student, grade, token_id);
    // Parameter::new("student", Key::cl_type()),
    // Parameter::new("grade", CLType::U32),
    // Parameter::new("token_id", TokenId::cl_type()),
}
/*
teachers can be removed, so they cant issue grades anymore
 */
#[no_mangle]
pub extern "C" fn remove_teacher() {
    let teacher: Key = runtime::get_named_arg("teacher");
    NotenContract::default().remove_teacher(teacher);
}
/*
teachers can be added, so they can issue grades
 */
#[no_mangle]
pub extern "C" fn add_teacher() {
    let teacher: Key = runtime::get_named_arg("teacher");
    NotenContract::default().add_teacher(teacher);
}

#[no_mangle]
pub extern "C" fn token_meta(){
    let token_id: TokenId = runtime::get_named_arg("token_id");
    NotenContract::default().token_meta(token_id);
}
/*
sets up the smart contract
 */
#[no_mangle]
pub extern "C" fn call() {
    // Read arguments for the constructor call.
    let name: String = runtime::get_named_arg("name");
    let symbol: String = runtime::get_named_arg("symbol");
    let meta: Meta = runtime::get_named_arg("meta");
    let contract_name: String = runtime::get_named_arg("contract_name");

    // Prepare constructor args
    let constructor_args:RuntimeArgs = runtime_args! {
        "name" => name,
        "symbol" => symbol,
        "meta" => meta
    };

    let (contract_hash, _) = storage::new_contract(
        get_entry_points(),
        None,
        Some(String::from("contract_package_hash")),
        None,
    );

    let package_hash: ContractPackageHash = ContractPackageHash::new(
        runtime::get_key("contract_package_hash")
            .unwrap_or_revert()
            .into_hash()
            .unwrap_or_revert(),
    );

    let constructor_access: URef =
        storage::create_contract_user_group(package_hash, "constructor", 1, Default::default())
            .unwrap_or_revert()
            .pop()
            .unwrap_or_revert();

    let _: () = runtime::call_contract(contract_hash, "constructor", constructor_args);

    let mut urefs = BTreeSet::new();
    urefs.insert(constructor_access);
    storage::remove_contract_user_group_urefs(package_hash, "constructor", urefs)
        .unwrap_or_revert();

    runtime::put_key(
        format!("{}_contract_hash", contract_name).as_str(),
        contract_hash.into(),
    );
    runtime::put_key(
        format!("{}_contract_hash_wrapped", contract_name).as_str(),
        storage::new_uref(contract_hash).into(),
    );
    runtime::put_key(
        format!("{}_package_hash_wrapped", contract_name).as_str(),
        storage::new_uref(package_hash).into(),
    );
}

fn get_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();
    entry_points.add_entry_point(EntryPoint::new(
        "constructor",
        vec![
            Parameter::new("name", String::cl_type()),
            Parameter::new("symbol", String::cl_type()),
            Parameter::new("meta", Meta::cl_type()),
        ],
        <()>::cl_type(),
        EntryPointAccess::Groups(vec![Group::new("constructor")]),
        EntryPointType::Contract,
    ));
    entry_points.add_entry_point(EntryPoint::new(
        "name",
        vec![],
        String::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));
    entry_points.add_entry_point(EntryPoint::new(
        "symbol",
        vec![],
        String::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));
    entry_points.add_entry_point(EntryPoint::new(
        "meta",
        vec![],
        Meta::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "token_meta",
        vec![Parameter::new("token_id", TokenId::cl_type())],
        Meta::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "grade",
        vec![
            Parameter::new("student", Key::cl_type()),
            Parameter::new("subject", CLType::String),
            Parameter::new("year", CLType::U32),
            Parameter::new("type", CLType::String),
            Parameter::new("grade", CLType::U32),
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "update_grade",
        vec![
            Parameter::new("student", Key::cl_type()),
            Parameter::new("grade", CLType::U32),
            Parameter::new("token_id", TokenId::cl_type()),

        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "add_teacher",
        vec![
            Parameter::new("teacher", Key::cl_type()),

        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "remove_teacher",
        vec![
            Parameter::new("teacher", Key::cl_type()),
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));


    entry_points
}


