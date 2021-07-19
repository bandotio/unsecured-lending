# unsecured-lending

Bandot unsecured lending protocol is implemented with Parity [Ink!](https://github.com/paritytech/ink), and developed with Patract [Redspot](https://github.com/patractlabs/redspot) tools.

Run the following command to install cargo-contract:

`$ cargo install cargo-contract --force --locked`,

or go to [cargo-contract](https://github.com/paritytech/cargo-contract) to learn more.

Compile your contract into wasm

`$ npx redspot compile`

Test your contract

`$ npx redspot test`

Open the interactive javascript console

`$ npx redspot console`

Get help information

`$ npx redspot help`

# Updates
For the most updated codes, please refer to the delegate branch :)

 We have created a simple credit delegation loan system by using the a rough KYC verification.

Try it from our test network: https://lend.bandot.io/

* Apply the DOT from the Patract Faucet: https://patrastore.io/jupiter-a1/system/accounts
* Click the KYC L1 Verification button on the Lend page at the Borrow section, then put on your email, name.
* As a user you can either be a lender to delegate DOT or a borrower to borrow DOT you got delegated from different lenders/delegators

For more detail, you can access our Github: https://github.com/bandotio/unsecured-lending

