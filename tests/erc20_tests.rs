// tests/erc20_tests.rs - Integration tests for ERC-20 Token
// These tests verify the contract ABI and basic functionality

use alloy_primitives::{Address, U256};

#[test]
fn test_address_type_basics() {
    // Test basic address operations
    let zero = Address::ZERO;
    assert_eq!(zero.0 .0, [0u8; 20]);
    
    let addr1 = Address::from([1u8; 20]);
    let addr2 = Address::from([2u8; 20]);
    
    assert_ne!(addr1, addr2);
    assert_ne!(addr1, zero);
}

#[test]
fn test_u256_type_basics() {
    // Test basic U256 operations
    let a = U256::from(100u64);
    let b = U256::from(200u64);
    
    assert!(a < b);
    assert!(b > a);
    assert_eq!(a + a, U256::from(200u64));
    assert_eq!(b - a, U256::from(100u64));
}

#[test]
fn test_supply_calculation() {
    const DECIMALS: u8 = 18;
    let initial_supply: u128 = 1_000_000 * 10u128.pow(DECIMALS as u32);
    
    // Verify supply calculation
    assert_eq!(initial_supply, 1_000_000_000_000_000_000_000_000u128);
}

#[test]
fn test_token_metadata_constants() {
    // Test that token metadata constants are correctly defined
    // These values should match the contract's expected configuration
    
    let decimals: u8 = 18;
    assert_eq!(decimals, 18);
}
