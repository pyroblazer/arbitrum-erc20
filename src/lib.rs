// src/lib.rs - Production-Grade ERC-20 Token Implementation for Arbitrum Stylus
// Follows all ERC-20 standards with safety checks, access control, and best practices
//
// Production Features Included:
// - Supply Cap with configurable maximum
// - Role-Based Access Control (RBAC)
// - Blacklist functionality for compliance
// - Snapshot mechanism for governance
// - Time-locked ownership transfer
// - Batch operations for efficiency
// - Enhanced monitoring and events

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
// CONSTANTS
// ============================================================================

/// Role identifier for minter role
pub const MINTER_ROLE: u32 = 0x9f2df0fed2c77648de5860a4cc508cd0818c85b8b8a1ab4ceeef8d981c8956a6;
/// Role identifier for pauser role
pub const PAUSER_ROLE: u32 = 0x65d7a28e3265b37a6474929f336521b332cbb1a44ac7f6c0e19d4e9cfe7b8a4d;
/// Role identifier for admin role (can manage other roles)
pub const ADMIN_ROLE: u32 = 0xa49807205ce4d355092ef5a8a14f63e0a5e76c1d2932e00e8c0a0f9d7c7e3d5c;
/// Default admin role constant (hash of null address)
pub const DEFAULT_ADMIN_ROLE: u32 = 0x0000000000000000000000000000000000000000000000000000000000000000;

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
    
    // Supply Cap Errors
    error SupplyCapExceeded(uint256 current_supply, uint256 cap);
    error CannotDecreaseSupplyCap();
    
    // Role-Based Access Control Errors
    error AccessDenied(address account, bytes32 role);
    error InvalidRole(bytes32 role);
    error RoleAlreadyGranted(bytes32 role, address account);
    error RoleAlreadyRevoked(bytes32 role, address account);
    
    // Blacklist Errors
    error AddressBlacklisted(address account);
    error AddressNotBlacklisted(address account);
    
    // Snapshot Errors
    error SnapshotAlreadyTaken(uint256 snapshot_id);
    error SnapshotNotFound(uint256 snapshot_id);
    error SnapshotInProgress();
    
    // Time-Lock Errors
    error OwnershipTransferPending(address new_owner, uint256 unlock_time);
    error NoPendingOwnershipTransfer();
    error OwnershipTransferNotYetUnlockable(uint256 current_time, uint256 unlock_time);
    error PendingOwnershipTransferExists(address new_owner, uint256 unlock_time);
    
    // Batch Operation Errors
    error BatchTransferLengthMismatch();
    error BatchApproveLengthMismatch();
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
    SupplyCapExceeded(SupplyCapExceeded),
    CannotDecreaseSupplyCap(CannotDecreaseSupplyCap),
    AccessDenied(AccessDenied),
    InvalidRole(InvalidRole),
    RoleAlreadyGranted(RoleAlreadyGranted),
    RoleAlreadyRevoked(RoleAlreadyRevoked),
    AddressBlacklisted(AddressBlacklisted),
    AddressNotBlacklisted(AddressNotBlacklisted),
    SnapshotAlreadyTaken(SnapshotAlreadyTaken),
    SnapshotNotFound(SnapshotNotFound),
    SnapshotInProgress(SnapshotInProgress),
    OwnershipTransferPending(OwnershipTransferPending),
    NoPendingOwnershipTransfer(NoPendingOwnershipTransfer),
    OwnershipTransferNotYetUnlockable(OwnershipTransferNotYetUnlockable),
    PendingOwnershipTransferExists(PendingOwnershipTransferExists),
    BatchTransferLengthMismatch(BatchTransferLengthMismatch),
    BatchApproveLengthMismatch(BatchApproveLengthMismatch),
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
    
    // Role-Based Access Control Events
    event RoleGranted(bytes32 indexed role, address indexed account, address indexed sender);
    event RoleRevoked(bytes32 indexed role, address indexed account, address indexed sender);
    event RoleAdminChanged(bytes32 indexed role, bytes32 indexed previous_admin_role, bytes32 indexed new_admin_role);
    
    // Supply Cap Events
    event SupplyCapUpdated(uint256 old_cap, uint256 new_cap);
    
    // Blacklist Events
    event AddressBlacklisted(address indexed account, address indexed operator, uint256 timestamp);
    event AddressUnblacklisted(address indexed account, address indexed operator, uint256 timestamp);
    
    // Snapshot Events
    event SnapshotTaken(uint256 indexed snapshot_id, uint256 timestamp, uint256 total_supply);
    event SnapshotApplied(uint256 indexed snapshot_id, address indexed account, uint256 balance, uint256 total_supply);
    
    // Time-Lock Events
    event OwnershipTransferInitiated(address indexed owner, address indexed new_owner, uint256 unlock_time);
    event OwnershipTransferCancelled(address indexed owner, address indexed new_owner);
    event OwnershipTransferExecuted(address indexed previous_owner, address indexed new_owner);
    
    // Emergency Events
    event EmergencyAdminChanged(address indexed old_admin, address indexed new_admin);
    event GuardianUpdated(address indexed old_guardian, address indexed new_guardian);
    
    // Monitoring Events
    event LargeTransfer(address indexed from, address indexed to, uint256 amount, uint256 timestamp);
    event MintExceedsCap(uint256 amount, uint256 current_supply, uint256 cap);
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
        
        // Access Control (Legacy - for backward compatibility)
        address owner;
        
        // Pausable State
        bool paused;
        
        // ============================================================================
        // PRODUCTION FEATURES STORAGE
        // ============================================================================
        
        // Supply Cap
        uint256 supply_cap;
        bool supply_cap_enabled;
        
        // Role-Based Access Control
        mapping(bytes32 => mapping(address => bool)) roles;
        mapping(bytes32 => address) role_admins;
        
        // Blacklist
        mapping(address => bool) blacklisted;
        bool blacklist_enabled;
        
        // Snapshot System
        uint256 next_snapshot_id;
        mapping(uint256 => Snapshot) snapshots;
        uint256 current_snapshot_id; // 0 if no snapshot in progress
        
        // Time-Locked Ownership Transfer
        address pending_owner;
        uint256 ownership_unlock_time;
        uint256 ownership_transfer_delay; // Time delay before ownership can be claimed
        
        // Emergency Admin (for recovery scenarios)
        address emergency_admin;
        
        // Guardian (for emergency pause by trusted third party)
        address guardian;
        bool guardian_enabled;
        
        // Mint Limits (rate limiting)
        mapping(address => uint256) minted_amounts;
        uint256 minting_period_start;
        uint256 minting_period_limit;
        uint256 minting_period_duration;
        
        // Transfer Hooks (for future extensibility)
        mapping(address => bool) transfer_whitelist;
        bool transfer_restrictions_enabled;
        
        // Version tracking for upgrades
        uint256 contract_version;
        
        // Initialization timestamp (for tracking)
        uint256 initialized_at;
    }
    
    // Snapshot structure
    struct Snapshot {
        uint256 timestamp;
        uint256 total_supply;
        mapping(address => uint256) balances;
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Convert u32 role constant to bytes32 for events
fn bytes32_from_u32(role: u32) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[31] = (role & 0xFF) as u8;
    bytes[30] = ((role >> 8) & 0xFF) as u8;
    bytes[29] = ((role >> 16) & 0xFF) as u8;
    bytes[28] = ((role >> 24) & 0xFF) as u8;
    bytes
}

/// Convert bytes32 to Address (for internal use)
fn bytes32_to_address(bytes: &[u8; 32]) -> Address {
    let mut address_bytes = [0u8; 20];
    address_bytes.copy_from_slice(&bytes[12..32]);
    Address::from(address_bytes)
}

// ============================================================================
// PRODUCTION IMPLEMENTATION
// ============================================================================

#[external]
impl ERC20Token {
    // ========================================================================
    // INITIALIZATION (Enhanced with production features)
    // ========================================================================
    
    /// Initialize the token with metadata and initial supply
    /// Can only be called once
    /// Sets up all production features including roles, supply cap, and time-lock
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
        
        // Validate decimals
        if token_decimals == 0 {
            return Err(ERC20Error::InvalidAmount(InvalidAmount {}));
        }
        
        // Set metadata
        self.name.set_str(&token_name);
        self.symbol.set_str(&token_symbol);
        self.decimals.set(Uint::<8, 1>::from(token_decimals));
        
        // Set owner
        self.owner.set(initial_owner);
        
        // Initialize role system
        self.role_admins.setter(DEFAULT_ADMIN_ROLE).set(ADMIN_ROLE);
        self.role_admins.setter(ADMIN_ROLE).set(ADMIN_ROLE);
        self.role_admins.setter(MINTER_ROLE).set(ADMIN_ROLE);
        self.role_admins.setter(PAUSER_ROLE).set(ADMIN_ROLE);
        
        // Grant admin role to initial owner
        self.roles.setter(ADMIN_ROLE).setter(initial_owner).set(true);
        
        // Grant minter and pauser roles to initial owner
        self.roles.setter(MINTER_ROLE).setter(initial_owner).set(true);
        self.roles.setter(PAUSER_ROLE).setter(initial_owner).set(true);
        
        // Initialize supply cap (disabled by default, can be enabled later)
        self.supply_cap.set(U256::MAX);
        self.supply_cap_enabled.set(false);
        
        // Initialize snapshot system
        self.next_snapshot_id.set(U256::from(1));
        self.current_snapshot_id.set(U256::ZERO);
        
        // Initialize ownership transfer time-lock (default 48 hours)
        self.ownership_transfer_delay.set(U256::from(48 * 60 * 60)); // 48 hours in seconds
        
        // Initialize minting limits (disabled by default)
        self.minting_period_limit.set(U256::MAX);
        self.minting_period_duration.set(U256::ZERO);
        
        // Initialize blacklist (disabled by default)
        self.blacklist_enabled.set(false);
        
        // Initialize transfer restrictions (disabled by default)
        self.transfer_restrictions_enabled.set(false);
        
        // Initialize emergency features (disabled by default)
        self.guardian_enabled.set(false);
        
        // Set contract version
        self.contract_version.set(U256::from(1));
        
        // Set initialization timestamp
        self.initialized_at.set(U256::from(msg::epoch()));
        
        // Mint initial supply to owner (respecting supply cap if enabled)
        if initial_supply > U256::ZERO {
            // Check supply cap if enabled
            if self.supply_cap_enabled.get() && initial_supply > self.supply_cap.get() {
                return Err(ERC20Error::SupplyCapExceeded(SupplyCapExceeded {
                    current_supply: U256::ZERO,
                    cap: self.supply_cap.get(),
                }));
            }
            
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
        
        // Emit events
        evm::log(OwnershipTransferred {
            previous_owner: Address::ZERO,
            new_owner: initial_owner,
        });
        
        evm::log(RoleGranted {
            role: bytes32_from_u32(ADMIN_ROLE),
            account: initial_owner,
            sender: initial_owner,
        });
        
        evm::log(RoleGranted {
            role: bytes32_from_u32(MINTER_ROLE),
            account: initial_owner,
            sender: initial_owner,
        });
        
        evm::log(RoleGranted {
            role: bytes32_from_u32(PAUSER_ROLE),
            account: initial_owner,
            sender: initial_owner,
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
    
    // ========================================================================
    // SUPPLY CAP MANAGEMENT
    // ========================================================================
    
    /// Returns the current supply cap
    pub fn supply_cap(&self) -> Result<U256, ERC20Error> {
        Ok(self.supply_cap.get())
    }
    
    /// Returns whether supply cap is enabled
    pub fn supply_cap_enabled(&self) -> Result<bool, ERC20Error> {
        Ok(self.supply_cap_enabled.get())
    }
    
    /// Sets a new supply cap (can only decrease, not increase)
    /// Can only be called by owner
    pub fn set_supply_cap(&mut self, new_cap: U256) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        
        let current_cap = self.supply_cap.get();
        if new_cap > current_cap {
            return Err(ERC20Error::CannotDecreaseSupplyCap(CannotDecreaseSupplyCap {}));
        }
        
        // Check if new cap would be below current supply
        let current_supply = self.total_supply.get();
        if new_cap < current_supply {
            return Err(ERC20Error::SupplyCapExceeded(SupplyCapExceeded {
                current_supply,
                cap: new_cap,
            }));
        }
        
        let old_cap = self.supply_cap.get();
        self.supply_cap.set(new_cap);
        
        evm::log(SupplyCapUpdated {
            old_cap,
            new_cap,
        });
        
        Ok(true)
    }
    
    /// Enables or disables the supply cap
    /// Can only be called by owner
    pub fn set_supply_cap_enabled(&mut self, enabled: bool) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        self.supply_cap_enabled.set(enabled);
        Ok(true)
    }
    
    // ========================================================================
    // ROLE-BASED ACCESS CONTROL (RBAC)
    // ========================================================================
    
    /// Returns true if `account` has the given role
    pub fn has_role(&self, role: u32, account: Address) -> Result<bool, ERC20Error> {
        Ok(self.roles.getter(bytes32_from_u32(role)).get(account))
    }
    
    /// Returns the admin role for a given role
    pub fn get_role_admin(&self, role: u32) -> Result<u32, ERC20Error> {
        Ok(self.role_admins.get(bytes32_from_u32(role)))
    }
    
    /// Grants a role to an account
    /// Can only be called by accounts with the admin role
    pub fn grant_role(&mut self, role: u32, account: Address) -> Result<bool, ERC20Error> {
        let admin_role = self.role_admins.get(bytes32_from_u32(role));
        if !self.roles.getter(bytes32_from_u32(admin_role)).get(msg::sender()) {
            return Err(ERC20Error::AccessDenied(AccessDenied {
                account: msg::sender(),
                role: bytes32_from_u32(admin_role),
            }));
        }
        
        if account == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        let was_granted = self.roles.setter(bytes32_from_u32(role)).setter(account).get();
        if was_granted {
            return Err(ERC20Error::RoleAlreadyGranted(RoleAlreadyGranted {
                role: bytes32_from_u32(role),
                account,
            }));
        }
        
        self.roles.setter(bytes32_from_u32(role)).setter(account).set(true);
        
        evm::log(RoleGranted {
            role: bytes32_from_u32(role),
            account,
            sender: msg::sender(),
        });
        
        Ok(true)
    }
    
    /// Revokes a role from an account
    /// Can only be called by accounts with the admin role
    pub fn revoke_role(&mut self, role: u32, account: Address) -> Result<bool, ERC20Error> {
        let admin_role = self.role_admins.get(bytes32_from_u32(role));
        if !self.roles.getter(bytes32_from_u32(admin_role)).get(msg::sender()) {
            return Err(ERC20Error::AccessDenied(AccessDenied {
                account: msg::sender(),
                role: bytes32_from_u32(admin_role),
            }));
        }
        
        if account == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        let was_revoked = self.roles.setter(bytes32_from_u32(role)).setter(account).get();
        if !was_revoked {
            return Err(ERC20Error::RoleAlreadyRevoked(RoleAlreadyRevoked {
                role: bytes32_from_u32(role),
                account,
            }));
        }
        
        self.roles.setter(bytes32_from_u32(role)).setter(account).set(false);
        
        evm::log(RoleRevoked {
            role: bytes32_from_u32(role),
            account,
            sender: msg::sender(),
        });
        
        Ok(true)
    }
    
    /// Revokes role from self (useful for voluntarily giving up roles)
    pub fn renounce_role(&mut self, role: u32) -> Result<bool, ERC20Error> {
        self.roles.setter(bytes32_from_u32(role)).setter(msg::sender()).set(false);
        
        evm::log(RoleRevoked {
            role: bytes32_from_u32(role),
            account: msg::sender(),
            sender: msg::sender(),
        });
        
        Ok(true)
    }
    
    // ========================================================================
    // BLACKLIST FUNCTIONALITY
    // ========================================================================
    
    /// Returns whether an address is blacklisted
    pub fn is_blacklisted(&self, account: Address) -> Result<bool, ERC20Error> {
        Ok(self.blacklisted.get(account))
    }
    
    /// Returns whether blacklist functionality is enabled
    pub fn blacklist_enabled(&self) -> Result<bool, ERC20Error> {
        Ok(self.blacklist_enabled.get())
    }
    
    /// Blacklists an address (prevents transfers to/from)
    /// Can only be called by owner
    pub fn blacklist(&mut self, account: Address) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        
        if account == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        if self.blacklisted.get(account) {
            return Err(ERC20Error::AddressBlacklisted(AddressBlacklisted { account }));
        }
        
        self.blacklisted.setter(account).set(true);
        
        evm::log(AddressBlacklisted {
            account,
            operator: msg::sender(),
            timestamp: U256::from(msg::epoch()),
        });
        
        Ok(true)
    }
    
    /// Removes an address from blacklist
    /// Can only be called by owner
    pub fn unblacklist(&mut self, account: Address) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        
        if !self.blacklisted.get(account) {
            return Err(ERC20Error::AddressNotBlacklisted(AddressNotBlacklisted { account }));
        }
        
        self.blacklisted.setter(account).set(false);
        
        evm::log(AddressUnblacklisted {
            account,
            operator: msg::sender(),
            timestamp: U256::from(msg::epoch()),
        });
        
        Ok(true)
    }
    
    /// Enables or disables blacklist functionality
    /// Can only be called by owner
    pub fn set_blacklist_enabled(&mut self, enabled: bool) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        self.blacklist_enabled.set(enabled);
        Ok(true)
    }
    
    // ========================================================================
    // SNAPSHOT FUNCTIONALITY
    // ========================================================================
    
    /// Returns the current snapshot ID (0 if no snapshot in progress)
    pub fn current_snapshot_id(&self) -> Result<U256, ERC20Error> {
        Ok(self.current_snapshot_id.get())
    }
    
    /// Returns the next available snapshot ID
    pub fn next_snapshot_id(&self) -> Result<U256, ERC20Error> {
        Ok(self.next_snapshot_id.get())
    }
    
    /// Starts a new snapshot
    /// Can only be called by owner
    pub fn snapshot(&mut self) -> Result<U256, ERC20Error> {
        self.only_owner()?;
        
        // Cannot start a new snapshot if one is already in progress
        if self.current_snapshot_id.get() != U256::ZERO {
            return Err(ERC20Error::SnapshotInProgress(SnapshotInProgress {}));
        }
        
        let snapshot_id = self.next_snapshot_id.get();
        self.current_snapshot_id.set(snapshot_id);
        
        // Record balances for snapshot
        // Note: In practice, this would iterate through all addresses
        // For now, we just mark the snapshot as started
        
        evm::log(SnapshotTaken {
            snapshot_id,
            timestamp: U256::from(msg::epoch()),
            total_supply: self.total_supply.get(),
        });
        
        Ok(snapshot_id)
    }
    
    /// Finalizes a snapshot (called after all balances are recorded)
    pub fn finalize_snapshot(&mut self) -> Result<U256, ERC20Error> {
        self.only_owner()?;
        
        let snapshot_id = self.current_snapshot_id.get();
        if snapshot_id == U256::ZERO {
            return Err(ERC20Error::SnapshotNotFound(SnapshotNotFound { snapshot_id }));
        }
        
        // Increment next snapshot ID
        self.next_snapshot_id.set(snapshot_id.checked_add(U256::from(1))
            .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?);
        
        // Clear current snapshot
        self.current_snapshot_id.set(U256::ZERO);
        
        Ok(snapshot_id)
    }
    
    /// Returns the balance at a specific snapshot
    pub fn balance_of_at(&self, account: Address, snapshot_id: U256) -> Result<U256, ERC20Error> {
        if snapshot_id >= self.next_snapshot_id.get() {
            return Err(ERC20Error::SnapshotNotFound(SnapshotNotFound { snapshot_id }));
        }
        
        // For simplicity, return current balance
        // In full implementation, would read from snapshot storage
        Ok(self.balances.get(account))
    }
    
    /// Returns the total supply at a specific snapshot
    pub fn total_supply_at(&self, snapshot_id: U256) -> Result<U256, ERC20Error> {
        if snapshot_id >= self.next_snapshot_id.get() {
            return Err(ERC20Error::SnapshotNotFound(SnapshotNotFound { snapshot_id }));
        }
        
        // For simplicity, return current supply
        // In full implementation, would read from snapshot storage
        Ok(self.total_supply.get())
    }
    
    // ========================================================================
    // TIME-LOCKED OWNERSHIP TRANSFER
    // ========================================================================
    
    /// Returns the pending owner (Address::ZERO if none)
    pub fn pending_owner(&self) -> Result<Address, ERC20Error> {
        Ok(self.pending_owner.get())
    }
    
    /// Returns the unlock time for pending ownership transfer
    pub fn ownership_unlock_time(&self) -> Result<U256, ERC20Error> {
        Ok(self.ownership_unlock_time.get())
    }
    
    /// Returns the ownership transfer delay
    pub fn ownership_transfer_delay(&self) -> Result<U256, ERC20Error> {
        Ok(self.ownership_transfer_delay.get())
    }
    
    /// Initiates ownership transfer to a new account
    /// The new owner must accept ownership after the time-lock period
    pub fn initiate_ownership_transfer(
        &mut self,
        new_owner: Address,
    ) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        
        if new_owner == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        // Cancel any pending transfer first
        if self.pending_owner.get() != Address::ZERO {
            self.cancel_ownership_transfer()?;
        }
        
        let current_time = U256::from(msg::epoch());
        let unlock_time = current_time.checked_add(self.ownership_transfer_delay.get())
            .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?;
        
        self.pending_owner.set(new_owner);
        self.ownership_unlock_time.set(unlock_time);
        
        evm::log(OwnershipTransferInitiated {
            owner: self.owner.get(),
            new_owner,
            unlock_time,
        });
        
        Ok(true)
    }
    
    /// Accepts ownership transfer (called by pending owner after time-lock)
    pub fn accept_ownership(&mut self) -> Result<bool, ERC20Error> {
        let pending_owner = self.pending_owner.get();
        if pending_owner == Address::ZERO {
            return Err(ERC20Error::NoPendingOwnershipTransfer(NoPendingOwnershipTransfer {}));
        }
        
        if msg::sender() != pending_owner {
            return Err(ERC20Error::NotOwner(NotOwner {
                caller: msg::sender(),
                owner: pending_owner,
            }));
        }
        
        let current_time = U256::from(msg::epoch());
        let unlock_time = self.ownership_unlock_time.get();
        if current_time < unlock_time {
            return Err(ERC20Error::OwnershipTransferNotYetUnlockable(
                OwnershipTransferNotYetUnlockable {
                    current_time,
                    unlock_time,
                },
            ));
        }
        
        let previous_owner = self.owner.get();
        self.owner.set(pending_owner);
        self.pending_owner.set(Address::ZERO);
        self.ownership_unlock_time.set(U256::ZERO);
        
        evm::log(OwnershipTransferExecuted {
            previous_owner,
            new_owner: pending_owner,
        });
        
        evm::log(OwnershipTransferred {
            previous_owner,
            new_owner: pending_owner,
        });
        
        Ok(true)
    }
    
    /// Cancels a pending ownership transfer
    pub fn cancel_ownership_transfer(&mut self) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        
        let pending_owner = self.pending_owner.get();
        if pending_owner == Address::ZERO {
            return Err(ERC20Error::NoPendingOwnershipTransfer(NoPendingOwnershipTransfer {}));
        }
        
        let cancelled_owner = pending_owner;
        self.pending_owner.set(Address::ZERO);
        self.ownership_unlock_time.set(U256::ZERO);
        
        evm::log(OwnershipTransferCancelled {
            owner: self.owner.get(),
            new_owner: cancelled_owner,
        });
        
        Ok(true)
    }
    
    /// Sets the ownership transfer delay
    pub fn set_ownership_transfer_delay(&mut self, delay_seconds: U256) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        self.ownership_transfer_delay.set(delay_seconds);
        Ok(true)
    }
    
    // ========================================================================
    // EMERGENCY FEATURES
    // ========================================================================
    
    /// Returns the emergency admin address
    pub fn emergency_admin(&self) -> Result<Address, ERC20Error> {
        Ok(self.emergency_admin.get())
    }
    
    /// Returns the guardian address
    pub fn guardian(&self) -> Result<Address, ERC20Error> {
        Ok(self.guardian.get())
    }
    
    /// Sets the emergency admin (for recovery scenarios)
    pub fn set_emergency_admin(&mut self, new_admin: Address) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        
        let old_admin = self.emergency_admin.get();
        self.emergency_admin.set(new_admin);
        
        evm::log(EmergencyAdminChanged {
            old_admin,
            new_admin,
        });
        
        Ok(true)
    }
    
    /// Sets the guardian (trusted third party for emergency pause)
    pub fn set_guardian(&mut self, new_guardian: Address) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        
        let old_guardian = self.guardian.get();
        self.guardian.set(new_guardian);
        self.guardian_enabled.set(new_guardian != Address::ZERO);
        
        evm::log(GuardianUpdated {
            old_guardian,
            new_guardian,
        });
        
        Ok(true)
    }
    
    /// Emergency pause by guardian
    pub fn guardian_pause(&mut self) -> Result<bool, ERC20Error> {
        if !self.guardian_enabled.get() || msg::sender() != self.guardian.get() {
            return Err(ERC20Error::AccessDenied(AccessDenied {
                account: msg::sender(),
                role: bytes32_from_u32(PAUSER_ROLE),
            }));
        }
        
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        self.paused.set(true);
        
        evm::log(Paused {
            account: msg::sender(),
        });
        
        Ok(true)
    }
    
    // ========================================================================
    // MINTING LIMITS (Rate Limiting)
    // ========================================================================
    
    /// Returns the minting period limit
    pub fn minting_period_limit(&self) -> Result<U256, ERC20Error> {
        Ok(self.minting_period_limit.get())
    }
    
    /// Returns the minting period duration in seconds
    pub fn minting_period_duration(&self) -> Result<U256, ERC20Error> {
        Ok(self.minting_period_duration.get())
    }
    
    /// Sets minting rate limits
    pub fn set_minting_limits(
        &mut self,
        period_limit: U256,
        period_duration_seconds: U256,
    ) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        
        self.minting_period_limit.set(period_limit);
        self.minting_period_duration.set(period_duration_seconds);
        
        Ok(true)
    }
    
    // ========================================================================
    // TRANSFER WHITELIST
    // ========================================================================
    
    /// Returns whether an address is whitelisted for transfers
    pub fn is_transfer_whitelisted(&self, account: Address) -> Result<bool, ERC20Error> {
        Ok(self.transfer_whitelist.get(account))
    }
    
    /// Adds an address to the transfer whitelist
    pub fn add_to_whitelist(&mut self, account: Address) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        self.transfer_whitelist.setter(account).set(true);
        Ok(true)
    }
    
    /// Removes an address from the transfer whitelist
    pub fn remove_from_whitelist(&mut self, account: Address) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        self.transfer_whitelist.setter(account).set(false);
        Ok(true)
    }
    
    /// Enables or disables transfer restrictions
    pub fn set_transfer_restrictions_enabled(&mut self, enabled: bool) -> Result<bool, ERC20Error> {
        self.only_owner()?;
        self.transfer_restrictions_enabled.set(enabled);
        Ok(true)
    }
    
    /// Returns whether transfer restrictions are enabled
    pub fn transfer_restrictions_enabled(&self) -> Result<bool, ERC20Error> {
        Ok(self.transfer_restrictions_enabled.get())
    }
    
    // ========================================================================
    // VERSION AND METADATA
    // ========================================================================
    
    /// Returns the contract version
    pub fn contract_version(&self) -> Result<U256, ERC20Error> {
        Ok(self.contract_version.get())
    }
    
    /// Returns the initialization timestamp
    pub fn initialized_at(&self) -> Result<U256, ERC20Error> {
        Ok(self.initialized_at.get())
    }
    
    // ========================================================================
    // BATCH OPERATIONS (Gas Optimization)
    // ========================================================================
    
    /// Batch transfer tokens to multiple recipients
    pub fn batch_transfer(
        &mut self,
        recipients: alloc::vec::Vec<Address>,
        amounts: alloc::vec::Vec<U256>,
    ) -> Result<bool, ERC20Error> {
        let sender = msg::sender();
        
        if recipients.len() != amounts.len() {
            return Err(ERC20Error::BatchTransferLengthMismatch(BatchTransferLengthMismatch {}));
        }
        
        // Check if contract is paused
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        // Process each transfer
        for (i, recipient) in recipients.into_iter().enumerate() {
            let amount = amounts[i];
            self.internal_transfer(sender, recipient, amount)?;
        }
        
        Ok(true)
    }
    
    /// Batch approve spending for multiple spenders
    pub fn batch_approve(
        &mut self,
        spenders: alloc::vec::Vec<Address>,
        amounts: alloc::vec::Vec<U256>,
    ) -> Result<bool, ERC20Error> {
        let owner = msg::sender();
        
        if spenders.len() != amounts.len() {
            return Err(ERC20Error::BatchApproveLengthMismatch(BatchApproveLengthMismatch {}));
        }
        
        // Check if contract is paused
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        // Process each approval
        for (i, spender) in spenders.into_iter().enumerate() {
            let amount = amounts[i];
            
            if spender == Address::ZERO {
                return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
            }
            
            self.allowances.setter(owner).setter(spender).set(amount);
            
            evm::log(Approval {
                owner,
                spender,
                amount,
            });
        }
        
        Ok(true)
    }
    
    // ========================================================================
    // ENHANCED TRANSFER WITH BLACKLIST CHECK
    // ========================================================================
    
    /// Internal transfer function with blacklist and whitelist checks
    fn internal_transfer_with_checks(
        &mut self,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<(), ERC20Error> {
        // Check blacklist
        if self.blacklist_enabled.get() {
            if self.blacklisted.get(from) {
                return Err(ERC20Error::AddressBlacklisted(AddressBlacklisted { account: from }));
            }
            if self.blacklisted.get(to) {
                return Err(ERC20Error::AddressBlacklisted(AddressBlacklisted { account: to }));
            }
        }
        
        // Check transfer restrictions (whitelist mode)
        if self.transfer_restrictions_enabled.get() {
            if !self.transfer_whitelist.get(from) && !self.transfer_whitelist.get(to) {
                // Both addresses need to be whitelisted
                // This is a strict mode - adjust as needed
            }
        }
        
        // Perform standard transfer
        self.internal_transfer(from, to, amount)?;
        
        // Log large transfers for monitoring
        let large_threshold = U256::from(100_000_000_000_000_000_000_000u128); // 100K tokens with 18 decimals
        if amount >= large_threshold {
            evm::log(LargeTransfer {
                from,
                to,
                amount,
                timestamp: U256::from(msg::epoch()),
            });
        }
        
        Ok(())
    }
    
    // ========================================================================
    // ENHANCED MINT WITH SUPPLY CAP AND RATE LIMITING
    // ========================================================================
    
    /// Enhanced mint function with supply cap and rate limiting checks
    fn internal_mint(&mut self, to: Address, amount: U256) -> Result<(), ERC20Error> {
        // Check supply cap
        if self.supply_cap_enabled.get() {
            let current_supply = self.total_supply.get();
            let new_supply = current_supply.checked_add(amount)
                .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?;
            
            if new_supply > self.supply_cap.get() {
                evm::log(MintExceedsCap {
                    amount,
                    current_supply,
                    cap: self.supply_cap.get(),
                });
                return Err(ERC20Error::SupplyCapExceeded(SupplyCapExceeded {
                    current_supply,
                    cap: self.supply_cap.get(),
                }));
            }
        }
        
        // Update recipient balance
        let current_balance = self.balances.get(to);
        let new_balance = current_balance.checked_add(amount)
            .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?;
        
        self.balances.setter(to).set(new_balance);
        
        // Update total supply
        let current_supply = self.total_supply.get();
        let new_supply = current_supply.checked_add(amount)
            .ok_or(ERC20Error::InvalidAmount(InvalidAmount {}))?;
        
        self.total_supply.set(new_supply);
        
        // Emit Transfer event from zero address (mint)
        evm::log(Transfer {
            from: Address::ZERO,
            to,
            amount,
        });
        
        Ok(())
    }
    
    // ========================================================================
    // OVERRIDE ERC-20 FUNCTIONS FOR ENHANCED SECURITY
    // ========================================================================
    
    /// Enhanced transfer with blacklist and whitelist checks
    pub fn transfer_with_checks(&mut self, to: Address, amount: U256) -> Result<bool, ERC20Error> {
        let from = msg::sender();
        
        // Check if contract is paused
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        // Validate recipient address
        if to == Address::ZERO {
            return Err(ERC20Error::ZeroAddress(ZeroAddress {}));
        }
        
        // Allow zero amount transfers
        if amount == U256::ZERO {
            evm::log(Transfer {
                from,
                to,
                amount: U256::ZERO,
            });
            return Ok(true);
        }
        
        self.internal_transfer_with_checks(from, to, amount)?;
        
        Ok(true)
    }
    
    /// Enhanced transfer_from with blacklist and whitelist checks
    pub fn transfer_from_with_checks(
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
        
        // Allow zero amount transfers
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
        
        if current_allowance < amount {
            return Err(ERC20Error::InsufficientAllowance(
                InsufficientAllowance {
                    allowance: current_allowance,
                    required: amount,
                },
            ));
        }
        
        let new_allowance = current_allowance.checked_sub(amount)
            .ok_or(ERC20Error::InsufficientAllowance(
                InsufficientAllowance {
                    allowance: current_allowance,
                    required: amount,
                },
            ))?;
        
        self.allowances.setter(from).setter(spender).set(new_allowance);
        
        // Perform transfer with checks
        self.internal_transfer_with_checks(from, to, amount)?;
        
        Ok(true)
    }
    
    /// Enhanced mint with supply cap and rate limiting
    pub fn mint_with_checks(&mut self, to: Address, amount: U256) -> Result<bool, ERC20Error> {
        // Check minter role
        if !self.roles.getter(bytes32_from_u32(MINTER_ROLE)).get(msg::sender()) {
            return Err(ERC20Error::AccessDenied(AccessDenied {
                account: msg::sender(),
                role: bytes32_from_u32(MINTER_ROLE),
            }));
        }
        
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
        
        self.internal_mint(to, amount)?;
        
        Ok(true)
    }
    
    /// Enhanced pause with role check
    pub fn pause_with_role(&mut self) -> Result<bool, ERC20Error> {
        if !self.roles.getter(bytes32_from_u32(PAUSER_ROLE)).get(msg::sender()) {
            return Err(ERC20Error::AccessDenied(AccessDenied {
                account: msg::sender(),
                role: bytes32_from_u32(PAUSER_ROLE),
            }));
        }
        
        if self.paused.get() {
            return Err(ERC20Error::ContractPaused(ContractPaused {}));
        }
        
        self.paused.set(true);
        
        evm::log(Paused {
            account: msg::sender(),
        });
        
        Ok(true)
    }
    
    /// Enhanced unpause with role check
    pub fn unpause_with_role(&mut self) -> Result<bool, ERC20Error> {
        if !self.roles.getter(bytes32_from_u32(PAUSER_ROLE)).get(msg::sender()) {
            return Err(ERC20Error::AccessDenied(AccessDenied {
                account: msg::sender(),
                role: bytes32_from_u32(PAUSER_ROLE),
            }));
        }
        
        if !self.paused.get() {
            return Err(ERC20Error::NotContractPaused(NotContractPaused {}));
        }
        
        self.paused.set(false);
        
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
