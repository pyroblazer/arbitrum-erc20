// src/lib.rs - Production-Grade ERC-20 Token Implementation for Arbitrum Stylus
// Follows all ERC-20 standards with safety checks, access control, and best practices

#![cfg_attr(all(not(feature = "export-abi"), not(test)), no_main)]
extern crate alloc;

use alloc::string::String;
use stylus_sdk::{
    alloy_primitives::{Address, U256, Uint},
    alloy_sol_types::sol,
    evm, msg,
    prelude::*,
};

// ============================================================================
// ERROR DEFINITIONS
// ============================================================================

sol! {
    // ERC-20 Standard Errors
    error InsufficientBalance(uint256 balance, uint256 required);
    error InsufficientAllowance(uint256 allowance, uint256 required);
    error ZeroAddress();
    error NotOwner(address caller, address owner);
    error AlreadyInitialized();
    error ContractPaused();
    error NotContractPaused();
    error InvalidAmount();
}

#[derive(SolidityError)]
pub enum ERC20Error {
    InsufficientBalance(InsufficientBalance),
    InsufficientAllowance(InsufficientAllowance),
    ZeroAddress(ZeroAddress),
    NotOwner(NotOwner),
    AlreadyInitialized(AlreadyInitialized),
    ContractPaused(ContractPaused),
    NotContractPaused(NotContractPaused),
    InvalidAmount(InvalidAmount),
}

// ============================================================================
// EVENT DEFINITIONS (EVM Compatible)
// ============================================================================

sol! {
    // ERC-20 Standard Events
    event Transfer(address indexed from, address indexed to, uint256 amount);
    event Approval(address indexed owner, address indexed spender, uint256 amount);
    
    // Additional Events for Access Control
    event OwnershipTransferred(address indexed previous_owner, address indexed new_owner);
    event Paused(address account);
    event Unpaused(address account);
}

// ============================================================================
// STORAGE LAYOUT
// ============================================================================

sol_storage! {
    #[entrypoint]
    pub struct ERC20Token {
        // ERC-20 Core State
        uint256 total_supply;
        mapping(address => uint256) balances;
        mapping(address => mapping(address => uint256)) allowances;
        
        // Token Metadata
        bool initialized;
        string name;
        string symbol;
        uint8 decimals;
        
        // Access Control
        address owner;
        
        // Pausable State
        bool paused;
    }
}

// ============================================================================
// IMPLEMENTATION
// ============================================================================

#[external]
impl ERC20Token {
    // ========================================================================
    // INITIALIZATION
    // ========================================================================
    
    /// Initialize the token with metadata and initial supply
    /// Can only be called once
    pub fn initialize(
        &mut self,
        token_name: String,
        token_symbol: String,
        token_decimals: u8,
        initial_supply: U256,
        initial_owner: Address,
    ) -> Result<(), ERC20Error> {
        // Check if already initialized
        if self.initialized.get() {
            return Err(ERC20Error::AlreadyInitialized(AlreadyInitialized {}));
        }
        
        // Validate owner address
        if initial_owner == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        // Set metadata
        self.name.set_str(&token_name);
        self.symbol.set_str(&token_symbol);
        self.decimals.set(Uint::<8, 1>::from(token_decimals));
        
        // Set owner
        self.owner.set(initial_owner);
        
        // Mint initial supply to owner
        if initial_supply > U256::ZERO {
            self.balances.setter(initial_owner).set(initial_supply);
            self.total_supply.set(initial_supply);
            
            // Emit Transfer event from zero address (mint)
            evm::log(Transfer {
                from: Address::ZERO,
                to: initial_owner,
                amount: initial_supply,
            });
        }
        
        // Mark as initialized
        self.initialized.set(true);
        
        // Emit ownership transfer from zero address
        evm::log(OwnershipTransferred {
            previous_owner: Address::ZERO,
            new_owner: initial_owner,
        });
        
        Ok(())
    }
    
    // ========================================================================
    // ERC-20 METADATA METHODS
    // ========================================================================
    
    /// Returns the name of the token
    pub fn name(&self) -> Result<String, ERC20Error> {
        Ok(self.name.get_string())
    }
    
    /// Returns the symbol of the token
    pub fn symbol(&self) -> Result<String, ERC20Error> {
        Ok(self.symbol.get_string())
    }
    
    /// Returns the number of decimals the token uses
    pub fn decimals(&self) -> Result<u8, ERC20Error> {
        Ok(self.decimals.get().to_le_bytes::<1>()[0])
    }
    
    // ========================================================================
    // ERC-20 CORE METHODS
    // ========================================================================
    
    /// Returns the total token supply
    pub fn total_supply(&self) -> Result<U256, ERC20Error> {
        Ok(self.total_supply.get())
    }
    
    /// Returns the account balance of another account with address `owner`
    pub fn balance_of(&self, owner: Address) -> Result<U256, ERC20Error> {
        Ok(self.balances.get(owner))
    }
    
    /// Transfers `amount` tokens to address `to`
    /// Returns true on success, reverts on failure
    pub fn transfer(&mut self, to: Address, amount: U256) -> Result<bool, ERC20Error> {
        let from = msg::sender();
        
        // Check if contract is paused
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        // Validate recipient address
        if to == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        // Allow zero amount transfers (ERC-20 compatible)
        if amount == U256::ZERO {
            // Still emit event for zero transfers
            evm::log(Transfer {
                from,
                to,
                amount: U256::ZERO,
            });
            return Ok(true);
        }
        
        // Execute transfer
        self.internal_transfer(from, to, amount)?;
        
        Ok(true)
    }
    
    /// Approves `spender` to spend `amount` tokens on behalf of caller
    /// Returns true on success, reverts on failure
    pub fn approve(&mut self, spender: Address, amount: U256) -> Result<bool, ERC20Error> {
        let owner = msg::sender();
        
        // Validate spender address (recommended best practice)
        if spender == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        // Set allowance
        self.allowances.setter(owner).setter(spender).set(amount);
        
        // Emit Approval event
        evm::log(Approval {
            owner,
            spender,
            amount,
        });
        
        Ok(true)
    }
    
    /// Returns the amount which `spender` is still allowed to withdraw from `owner`
    pub fn allowance(&self, owner: Address, spender: Address) -> Result<U256, ERC20Error> {
        Ok(self.allowances.getter(owner).get(spender))
    }
    
    /// Transfers `amount` tokens from address `from` to address `to`
    /// The caller must have allowance for `from`'s tokens of at least `amount`
    /// Returns true on success, reverts on failure
    pub fn transfer_from(
        &mut self,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<bool, ERC20Error> {
        let spender = msg::sender();
        
        // Check if contract is paused
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        // Validate recipient address
        if to == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        // Allow zero amount transfers (ERC-20 compatible)
        if amount == U256::ZERO {
            evm::log(Transfer {
                from,
                to,
                amount: U256::ZERO,
            });
            return Ok(true);
        }
        
        // Check and update allowance
        let current_allowance = self.allowances.getter(from).get(spender);
        
        // Check for sufficient allowance
        if current_allowance < amount {
            return Err(ERC20Error::InsufficientAllowance(
                InsufficientAllowance {
                    allowance: current_allowance,
                    required: amount,
                },
            ));
        }
        
        // Decrease allowance using checked subtraction
        let new_allowance = current_allowance
            .checked_sub(amount)
            .ok_or(ERC20Error::InsufficientAllowance(
                InsufficientAllowance {
                    allowance: current_allowance,
                    required: amount,
                },
            ))?;
        
        self.allowances.setter(from).setter(spender).set(new_allowance);
        
        // Execute transfer
        self.internal_transfer(from, to, amount)?;
        
        Ok(true)
    }
    
    // ========================================================================
    // INTERNAL TRANSFER METHOD
    // ========================================================================
    
    /// Internal function to execute token transfer
    fn internal_transfer(
        &mut self,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<(), ERC20Error> {
        let from_balance = self.balances.get(from);
        
        // Check sufficient balance
        if from_balance < amount {
            return Err(ERC20Error::InsufficientBalance(InsufficientBalance {
                balance: from_balance,
                required: amount,
            }));
        }
        
        // Update balances with checked arithmetic
        let new_from_balance = from_balance
            .checked_sub(amount)
            .ok_or(ERC20Error::InsufficientBalance(InsufficientBalance {
                balance: from_balance,
                required: amount,
            }))?;
        
        let to_balance = self.balances.get(to);
        let new_to_balance = to_balance
            .checked_add(amount)
            .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?;
        
        self.balances.setter(from).set(new_from_balance);
        self.balances.setter(to).set(new_to_balance);
        
        // Emit transfer event
        evm::log(Transfer { from, to, amount });
        
        Ok(())
    }
    
    // ========================================================================
    // SAFE ALLOWANCE METHODS (Mitigates race condition)
    // ========================================================================
    
    /// Atomically increases the allowance granted to `spender` by the caller
    /// Mitigates the allowance race condition vulnerability
    pub fn increase_allowance(
        &mut self,
        spender: Address,
        delta: U256,
    ) -> Result<bool, ERC20Error> {
        let owner = msg::sender();
        
        // Validate spender address
        if spender == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        // Get current allowance
        let current_allowance = self.allowances.getter(owner).get(spender);
        
        // Calculate new allowance with overflow check
        let new_allowance = current_allowance
            .checked_add(delta)
            .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?;
        
        // Set new allowance
        self.allowances.setter(owner).setter(spender).set(new_allowance);
        
        // Emit Approval event
        evm::log(Approval {
            owner,
            spender,
            amount: new_allowance,
        });
        
        Ok(true)
    }
    
    /// Atomically decreases the allowance granted to `spender` by the caller
    /// Mitigates the allowance race condition vulnerability
    pub fn decrease_allowance(
        &mut self,
        spender: Address,
        delta: U256,
    ) -> Result<bool, ERC20Error> {
        let owner = msg::sender();
        
        // Validate spender address
        if spender == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        // Get current allowance
        let current_allowance = self.allowances.getter(owner).get(spender);
        
        // Check for sufficient allowance
        if current_allowance < delta {
            return Err(ERC20Error::InsufficientAllowance(
                InsufficientAllowance {
                    allowance: current_allowance,
                    required: delta,
                },
            ));
        }
        
        // Calculate new allowance with underflow check
        let new_allowance = current_allowance
            .checked_sub(delta)
            .ok_or(ERC20Error::InsufficientAllowance(
                InsufficientAllowance {
                    allowance: current_allowance,
                    required: delta,
                },
            ))?;
        
        // Set new allowance
        self.allowances.setter(owner).setter(spender).set(new_allowance);
        
        // Emit Approval event
        evm::log(Approval {
            owner,
            spender,
            amount: new_allowance,
        });
        
        Ok(true)
    }
    
    // ========================================================================
    // MINTABLE FUNCTIONALITY (Owner Only)
    // ========================================================================
    
    /// Mints `amount` tokens to address `to`
    /// Can only be called by the owner
    pub fn mint(&mut self, to: Address, amount: U256) -> Result<bool, ERC20Error> {
        // Check ownership
        self.only_owner()?;
        
        // Check if contract is paused
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        // Validate recipient address
        if to == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        // Skip if amount is zero
        if amount == U256::ZERO {
            return Ok(true);
        }
        
        // Update recipient balance with overflow check
        let current_balance = self.balances.get(to);
        let new_balance = current_balance
            .checked_add(amount)
            .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?;
        
        self.balances.setter(to).set(new_balance);
        
        // Update total supply with overflow check
        let current_supply = self.total_supply.get();
        let new_supply = current_supply
            .checked_add(amount)
            .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?;
        
        self.total_supply.set(new_supply);
        
        // Emit Transfer event from zero address (mint)
        evm::log(Transfer {
            from: Address::ZERO,
            to,
            amount,
        });
        
        Ok(true)
    }
    
    // ========================================================================
    // BURNABLE FUNCTIONALITY
    // ========================================================================
    
    /// Burns `amount` tokens from the caller's account
    pub fn burn(&mut self, amount: U256) -> Result<bool, ERC20Error> {
        let from = msg::sender();
        
        // Check if contract is paused
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        // Skip if amount is zero
        if amount == U256::ZERO {
            return Ok(true);
        }
        
        // Check balance
        let current_balance = self.balances.get(from);
        if current_balance < amount {
            return Err(ERC20Error::InsufficientBalance(InsufficientBalance {
                balance: current_balance,
                required: amount,
            }));
        }
        
        // Update balance with underflow check
        let new_balance = current_balance
            .checked_sub(amount)
            .ok_or(ERC20Error::InsufficientBalance(InsufficientBalance {
                balance: current_balance,
                required: amount,
            }))?;
        
        // Update total supply
        let current_supply = self.total_supply.get();
        let new_supply = current_supply
            .checked_sub(amount)
            .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?;
        
        self.balances.setter(from).set(new_balance);
        self.total_supply.set(new_supply);
        
        // Emit Transfer event to zero address (burn)
        evm::log(Transfer {
            from,
            to: Address::ZERO,
            amount,
        });
        
        Ok(true)
    }
    
    /// Burns `amount` tokens from `from` account on behalf of the caller
    /// The caller must have allowance for `from`'s tokens of at least `amount`
    pub fn burn_from(&mut self, from: Address, amount: U256) -> Result<bool, ERC20Error> {
        let spender = msg::sender();
        
        // Check if contract is paused
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        // Validate from address
        if from == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        // Skip if amount is zero
        if amount == U256::ZERO {
            return Ok(true);
        }
        
        // Check and update allowance
        let current_allowance = self.allowances.getter(from).get(spender);
        
        // Check for sufficient allowance
        if current_allowance < amount {
            return Err(ERC20Error::InsufficientAllowance(
                InsufficientAllowance {
                    allowance: current_allowance,
                    required: amount,
                },
            ));
        }
        
        // Decrease allowance using checked subtraction
        let new_allowance = current_allowance
            .checked_sub(amount)
            .ok_or(ERC20Error::InsufficientAllowance(
                InsufficientAllowance {
                    allowance: current_allowance,
                    required: amount,
                },
            ))?;
        
        self.allowances.setter(from).setter(spender).set(new_allowance);
        
        // Check balance and burn
        let current_balance = self.balances.get(from);
        if current_balance < amount {
            return Err(ERC20Error::InsufficientBalance(InsufficientBalance {
                balance: current_balance,
                required: amount,
            }));
        }
        
        // Update balance with underflow check
        let new_balance = current_balance
            .checked_sub(amount)
            .ok_or(ERC20Error::InsufficientBalance(InsufficientBalance {
                balance: current_balance,
                required: amount,
            }))?;
        
        // Update total supply
        let current_supply = self.total_supply.get();
        let new_supply = current_supply
            .checked_sub(amount)
            .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?;
        
        self.balances.setter(from).set(new_balance);
        self.total_supply.set(new_supply);
        
        // Emit Transfer event to zero address (burn)
        evm::log(Transfer {
            from,
            to: Address::ZERO,
            amount,
        });
        
        Ok(true)
    }
    
    // ========================================================================
    // OWNERSHIP MANAGEMENT
    // ========================================================================
    
    /// Returns the current owner of the contract
    pub fn owner(&self) -> Result<Address, ERC20Error> {
        Ok(self.owner.get())
    }
    
    /// Transfers ownership of the contract to a new account (`new_owner`)
    /// Can only be called by the current owner
    pub fn transfer_ownership(
        &mut self,
        new_owner: Address,
    ) -> Result<bool, ERC20Error> {
        // Check ownership
        self.only_owner()?;
        
        // Validate new owner address
        if new_owner == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        let previous_owner = self.owner.get();
        
        self.owner.set(new_owner);
        
        // Emit ownership transfer event
        evm::log(OwnershipTransferred {
            previous_owner,
            new_owner,
        });
        
        Ok(true)
    }
    
    /// Internal function to check if caller is owner
    fn only_owner(&self) -> Result<(), ERC20Error> {
        let caller = msg::sender();
        let owner = self.owner.get();
        
        if caller != owner {
            return Err(ERC20Error::NotOwner(NotOwner { caller, owner }));
        }
        
        Ok(())
    }
    
    /// Leaves the contract without an owner
    /// After renouncing ownership, owner will be Address::ZERO
    /// Cannot be called if the current owner is Address::ZERO
    pub fn renounce_ownership(&mut self) -> Result<bool, ERC20Error> {
        // Check ownership
        self.only_owner()?;
        
        let previous_owner = self.owner.get();
        
        // Set owner to zero address
        self.owner.set(Address::ZERO);
        
        // Emit ownership transfer event
        evm::log(OwnershipTransferred {
            previous_owner,
            new_owner: Address::ZERO,
        });
        
        Ok(true)
    }
    
    // ========================================================================
    // PAUSABLE FUNCTIONALITY
    // ========================================================================
    
    /// Returns true if the contract is paused, false otherwise
    pub fn paused(&self) -> Result<bool, ERC20Error> {
        Ok(self.paused.get())
    }
    
    /// Pauses the contract
    /// Can only be called by the owner
    pub fn pause(&mut self) -> Result<bool, ERC20Error> {
        // Check ownership
        self.only_owner()?;
        
        // Check if already paused
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        self.paused.set(true);
        
        // Emit Paused event
        evm::log(Paused {
            account: msg::sender(),
        });
        
        Ok(true)
    }
    
    /// Unpauses the contract
    /// Can only be called by the owner
    pub fn unpause(&mut self) -> Result<bool, ERC20Error> {
        // Check ownership
        self.only_owner()?;
        
        // Check if already unpaused
        if !self.paused.get() {
            return Err(ERC20Error::NotContractPaused(NotContractPaused {}));
        }
        
        self.paused.set(false);
        
        // Emit Unpaused event
        evm::log(Unpaused {
            account: msg::sender(),
        });
        
        Ok(true)
    }
}

// ============================================================================
// UNIT TESTS
// Note: Full contract tests require Stylus VM and are in tests/erc20_tests.rs
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, U256};

    // Helper function to create test addresses
    fn addr(n: u8) -> Address {
        Address::from([n; 20])
    }

    // Test constants
    const DECIMALS: u8 = 18;
    const INITIAL_SUPPLY: u128 = 1_000_000 * 10u128.pow(DECIMALS as u32);

    // ============================================================================
    // BASIC TYPE AND OPERATION TESTS
    // ============================================================================

    #[test]
    fn test_address_operations() {
        // Test address creation and comparison
        let zero = Address::ZERO;
        assert_eq!(zero.0 .0, [0u8; 20]);

        let addr1 = addr(1);
        let addr2 = addr(2);

        assert_ne!(addr1, addr2);
        assert_ne!(addr1, zero);
        assert_ne!(addr2, zero);

        // Test ordering
        assert!(addr1 < addr2);
        assert!(zero < addr1);
    }

    #[test]
    fn test_u256_operations() {
        // Test U256 arithmetic operations
        let supply = U256::from(INITIAL_SUPPLY);
        let transfer_amount = U256::from(1000u64);

        // Test subtraction
        let remaining = supply - transfer_amount;
        assert_eq!(remaining, supply - transfer_amount);

        // Test addition
        let mint_amount = U256::from(500u64);
        let new_supply = supply + mint_amount;
        assert_eq!(new_supply, supply + mint_amount);

        // Test comparison
        assert!(supply > transfer_amount);
        assert!(transfer_amount > mint_amount);
    }

    #[test]
    fn test_decimals_constant() {
        assert_eq!(DECIMALS, 18);
        let expected_supply = 1_000_000 * 10u128.pow(18);
        assert_eq!(INITIAL_SUPPLY, expected_supply);
    }

    #[test]
    fn test_u256_from_types() {
        let from_u8 = U256::from(42u8);
        let from_u16 = U256::from(42u16);
        let from_u32 = U256::from(42u32);
        let from_u64 = U256::from(42u64);
        let from_u128 = U256::from(42u128);

        assert_eq!(from_u8, from_u16);
        assert_eq!(from_u16, from_u32);
        assert_eq!(from_u32, from_u64);
        assert_eq!(from_u64, from_u128);
    }

    #[test]
    fn test_u256_zero_check() {
        let zero = U256::ZERO;
        let non_zero = U256::from(100u64);

        assert!(zero == U256::ZERO);
        assert!(!(non_zero == U256::ZERO));
    }

    #[test]
    fn test_address_from_slice() {
        let empty = Address::from([0u8; 20]);
        assert_eq!(empty, Address::ZERO);

        let addr1 = Address::from([1u8; 20]);
        let addr255 = Address::from([255u8; 20]);

        assert_ne!(addr1, addr255);
        assert_ne!(addr1, empty);
    }

    #[test]
    fn test_error_types_compile() {
        // Verify error types are properly defined
        let _insufficient_balance = ERC20Error::InsufficientBalance(InsufficientBalance {
            balance: U256::ZERO,
            required: U256::from(100),
        });

        let _insufficient_allowance = ERC20Error::InsufficientAllowance(InsufficientAllowance {
            allowance: U256::ZERO,
            required: U256::from(100),
        });

        let _zero_address = ERC20Error::ZeroAddress(ZeroAddress {});

        let _not_owner = ERC20Error::NotOwner(NotOwner {
            caller: Address::ZERO,
            owner: Address::ZERO,
        });
    }

    #[test]
    fn test_event_types_compile() {
        // Verify event types are properly defined
        let transfer_event = Transfer {
            from: Address::ZERO,
            to: addr(1),
            amount: U256::from(1000),
        };

        let approval_event = Approval {
            owner: addr(1),
            spender: addr(2),
            amount: U256::from(500),
        };

        let ownership_event = OwnershipTransferred {
            previous_owner: Address::ZERO,
            new_owner: addr(1),
        };

        // Events should be constructible
        assert_ne!(transfer_event.from, transfer_event.to);
        assert_ne!(approval_event.owner, approval_event.spender);
        assert_eq!(ownership_event.previous_owner, Address::ZERO);
    }

    // ============================================================================
    // SUPPLY CALCULATION TESTS
    // ============================================================================

    #[test]
    fn test_supply_calculations() {
        // Test with 18 decimals
        let decimals_18: u8 = 18;
        let supply_18: u128 = 1_000_000 * 10u128.pow(decimals_18 as u32);
        assert_eq!(supply_18, 1_000_000_000_000_000_000_000_000u128);

        // Test with 6 decimals
        let decimals_6: u8 = 6;
        let supply_6: u128 = 1_000_000 * 10u128.pow(decimals_6 as u32);
        assert_eq!(supply_6, 1_000_000_000_000u128);

        // Large supply
        let large_supply: u128 = 1_000_000_000 * 10u128.pow(18);
        assert_eq!(large_supply, 1_000_000_000_000_000_000_000_000_000u128);
    }

    // ============================================================================
    // U256 ARITHMETIC TESTS
    // ============================================================================

    #[test]
    fn test_u256_checked_add() {
        // u64::MAX is within U256 range, so adding 1 won't overflow U256
        let a = U256::from(u64::MAX);
        let b = U256::from(1u64);
        let result = a.checked_add(b);
        assert!(result.is_some()); // No overflow in U256

        let c = U256::from(100u64);
        let d = U256::from(200u64);
        let result = c.checked_add(d);
        assert_eq!(result, Some(U256::from(300u64)));
    }

    #[test]
    fn test_u256_checked_sub() {
        let a = U256::from(100u64);
        let b = U256::from(50u64);
        let result = a.checked_sub(b);
        assert_eq!(result, Some(U256::from(50u64)));

        let c = U256::from(50u64);
        let d = U256::from(100u64);
        let result = c.checked_sub(d);
        assert!(result.is_none()); // Underflow
    }

    #[test]
    fn test_u256_max_values() {
        let max = U256::MAX;
        let one = U256::from(1u64);
        let result = max.checked_add(one);
        assert!(result.is_none()); // Overflow

        let result = max.checked_sub(one);
        assert!(result.is_some());
    }

    // ============================================================================
    // ADDRESS COMPARISON TESTS
    // ============================================================================

    #[test]
    fn test_address_comparison() {
        let a = addr(1);
        let b = addr(2);
        let zero = Address::ZERO;

        assert!(a < b);
        assert!(a > zero);
        assert!(zero < a);
        assert!(a != b);
        assert!(a == a);
    }

    // ============================================================================
    // TOKEN METADATA VALIDATION TESTS
    // ============================================================================

    #[test]
    fn test_token_decimals_validation() {
        // Valid decimals (18^1 = 18, 18^2 = 324 which is > 255 for u8)
        assert_eq!(18u8.pow(1), 18);

        // Supply calculations (10^18 = 1 followed by 18 zeros)
        let supply = 1_000_000u128;
        let decimals = 18;
        let expected = supply * 10u128.pow(decimals);
        assert_eq!(expected, 1_000_000_000_000_000_000_000_000u128);
    }
}
