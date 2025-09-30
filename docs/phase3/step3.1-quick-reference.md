# Step 3.1 - Quick Fix Reference Card

## 🔧 5 Critical Fixes Required

---

### Fix 1: Borrow Checker (E0382) - 5 locations
**File**: `backend/src/services/mpc.rs`  
**Lines**: 888, 911, 934, 957, 980

```rust
// ❌ BEFORE
let status = response.status();
if !status.is_success() {
    let error_text = response.text().await...

// ✅ AFTER  
let status = response.status(); // Save first

if !status.is_success() {
    let error_text = response.text().await... // Then consume
```

**Apply to**:
- `send_keygen_request`
- `send_aggregate_request`
- `send_sign_phase1_request`
- `send_sign_phase2_request`
- `send_aggregate_signature_request`

---

### Fix 2: Serialize MpcError (E0277)
**File**: `backend/src/services/mpc.rs`  
**Line**: ~32

```rust
// ❌ BEFORE
#[derive(Error, Debug)]
pub enum MpcError {
    RequestFailed(#[from] reqwest::Error),

// ✅ AFTER
#[derive(Error, Debug, Serialize, Deserialize, Clone)]
pub enum MpcError {
    RequestFailed(String), // Changed type
```

**Add at end of file**:
```rust
impl From<reqwest::Error> for MpcError {
    fn from(err: reqwest::Error) -> Self {
        MpcError::RequestFailed(err.to_string())
    }
}
```

---

### Fix 3: ClusterStatus Field (E0609)
**File**: `backend/src/services/wallet_service.rs`  
**Line**: 208

```rust
// ❌ BEFORE
"cluster_operational": cluster_status.is_operational,

// ✅ AFTER
"cluster_operational": cluster_status.threshold_met,
```

---

### Fix 4: Create lib.rs (E0433)
**File**: `backend/src/lib.rs` (NEW FILE)

```rust
// Create this file:
pub mod services;
pub mod routes;
pub mod middleware;
pub mod models;
pub mod error;

pub use services::mpc::{MpcClient, MpcError};
pub use models::*;
```

**Update**: `backend/Cargo.toml`
```toml
[lib]
name = "backend"
path = "src/lib.rs"

[[bin]]
name = "backend"
path = "src/main.rs"
```

---

### Fix 5: Rate Limit Types (E0308) - Optional
**File**: `backend/src/middleware/rate_limit.rs`  
**Line**: 122

```rust
// Keep generic type B
type Response = ServiceResponse<B>; // Not BoxBody

// OR add conversion:
.map_into_boxed_body()
```

---

## 🚀 Quick Apply

### Automated (Recommended)
```bash
chmod +x apply_step_3_1_fixes.sh
./apply_step_3_1_fixes.sh
```

### Manual
1. Open each file in your editor
2. Apply fixes in order (1→5)
3. Save all files
4. Run: `cargo check`

---

## ✅ Verification

```bash
cd backend

# Should pass with 0 errors
cargo check

# Should compile successfully  
cargo build

# Ready for testing
cargo test --test test_step_3_1_complete --no-run
```

---

## 📊 Error Count

| Error | Count | Priority |
|-------|-------|----------|
| E0382 | 5     | 🔴 High  |
| E0277 | 1     | 🔴 High  |
| E0609 | 1     | 🟡 Medium|
| E0433 | 1     | 🟡 Medium|
| E0308 | 1     | 🟢 Low   |

---

## ⏱️ Time Estimate

- **Automated**: 2-3 minutes
- **Manual**: 10-15 minutes

---

## 🆘 Emergency Commands

```bash
# Revert all changes
git checkout backend/src/services/mpc.rs
git checkout backend/src/services/wallet_service.rs

# Start fresh
cargo clean
cargo build

# See all errors
cargo check 2>&1 | grep "error\["
```

---

**Print this card** and keep it handy while fixing!