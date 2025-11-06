//! Arrow Functions Converter
//!
//! This module is responsible for transforming arrow functions (`() => {}`) to
//! function expressions (`function () {}`).
//!
//! ## Note
//!
//! The OXC version of this converter is deeply integrated with OXC's traverse
//! system, semantic analysis, and arena allocator. A direct port to SWC is not
//! feasible. Instead, SWC already has built-in support for arrow function
//! conversion through `swc_ecma_transforms_compat::es2015::arrow`.
//!
//! ## Example
//!
//! Input:
//! ```js
//! var a = () => {};
//! var a = b => b;
//!
//! const double = [1, 2, 3].map(num => num * 2);
//! console.log(double); // [2,4,6]
//!
//! var bob = {
//!   name: "Bob",
//!   friends: ["Sally", "Tom"],
//!   printFriends() {
//!     this.friends.forEach(f => console.log(this.name + " knows " + f));
//!   },
//! };
//! console.log(bob.printFriends());
//! ```
//!
//! Output:
//! ```js
//! var a = function() {};
//! var a = function(b) { return b; };
//!
//! const double = [1, 2, 3].map(function(num) {
//!   return num * 2;
//! });
//! console.log(double); // [2,4,6]
//!
//! var bob = {
//!   name: "Bob",
//!   friends: ["Sally", "Tom"],
//!   printFriends() {
//!     var _this = this;
//!     this.friends.forEach(function(f) {
//!       return console.log(_this.name + " knows " + f);
//!     });
//!   },
//! };
//! console.log(bob.printFriends());
//! ```
//!
//! ## SWC Integration
//!
//! For SWC-based arrow function conversion, use:
//! - `swc_ecma_transforms_compat::es2015::arrow` for standard arrow function
//!   conversion
//! - The conversion is handled automatically when targeting ES5 or when arrow
//!   functions are not supported
//!
//! ## Implementation Notes
//!
//! The OXC implementation includes:
//! - Mode selection (Disabled, Enabled, AsyncOnly)
//! - `this` binding transformation with temp variable creation
//! - `arguments` binding transformation
//! - `super` method call handling in classes
//! - JSX element name transformation
//! - Complex scope tracking with sparse stacks
//!
//! All of these features are handled differently in SWC's architecture using
//! VisitMut pattern rather than OXC's Traverse pattern.

/// Mode for arrow function conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ArrowFunctionConverterMode {
    /// Disable arrow function conversion
    Disabled,

    /// Convert all arrow functions to regular functions
    Enabled,

    /// Only convert async arrow functions
    AsyncOnly,
}

// Note: The actual implementation should use SWC's built-in arrow function
// transform or implement a custom VisitMut visitor that follows SWC's patterns.
