use casper_types::U256;
use crate::contract::NotenContract;

mod contract;

#[test]
fn deploy() {
    NotenContract::deploy_noten();
}

#[test]
#[should_panic]
fn add_teacher_by_non_admin() {
    let mut contract = NotenContract::deploy_noten();
    let (_admin, _biff, tim, ali, _bob, _dan) = contract.accounts;

    contract.add_teacher(&tim, ali);
}

#[test]
fn add_teacher_by_admin() {
    let mut contract = NotenContract::deploy_noten();
    let (admin, _biff, _tim, ali, _bob, _dan) = contract.accounts;

    contract.add_teacher(&admin, ali);
}

#[test]
fn add_teacher_by_teacher() {
    let mut contract = NotenContract::deploy_noten();
    let (admin, _biff, tim, ali, _bob, _dan) = contract.accounts;

    contract.add_teacher(&admin, ali);
    contract.add_teacher(&ali, tim);
}

#[test]
#[should_panic]
fn remove_teacher_by_admin() {
    let mut contract = NotenContract::deploy_noten();
    let (admin, _biff, tim, ali, _bob, _dan) = contract.accounts;

    contract.add_teacher(&admin, ali);
    contract.remove_teacher(&admin, ali);
    contract.add_teacher(&ali, tim);
}


#[test]
#[should_panic]
fn remove_teacher_by_non_admin() {
    let mut contract = NotenContract::deploy_noten();
    let (admin, _biff, tim, ali, _bob, _dan) = contract.accounts;

    contract.add_teacher(&admin, ali);
    contract.remove_teacher(&tim, ali);
}

#[test]
fn teacher_gives_grade() {
    let mut contract = NotenContract::deploy_noten();
    let (admin, _biff, _tim, ali, bob, _dan) = contract.accounts;

    contract.add_teacher(&admin, ali);
    contract.grade(&ali, bob, "maths".to_string(), 4, "project".to_string(), 30);
    let token = contract.get_token_by_index(bob, U256::zero());
    assert!(token.is_some());
}

#[test]
#[should_panic]
fn student_gives_grade() {
    let mut contract = NotenContract::deploy_noten();
    let (admin, _biff, _tim, ali, bob, dan) = contract.accounts;

    contract.add_teacher(&admin, ali);
    contract.grade(&dan, bob, "maths".to_string(), 4, "project".to_string(), 30);
}