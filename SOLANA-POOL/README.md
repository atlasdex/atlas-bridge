# Main Function
 - Create Liquidity Pool, token LP A-B
 - Provide Liquidity
 - Withdraw 
 - Provide the status of Liquidity Pool
 - Total fee is 0.3%, 0.2 should be reinjected to the pool and 0.1% on a fixed fees account

## Building

To build a development version of the Token Swap program, you can use the normal
build command for Solana programs:

```sh
cd ./js
```
```sh
npm run build:program
```
```sh
npm i
```
```sh
npm run start-with-test-validator
```
```sh
solana program deploy ../target/deploy/amm_cropper.so
```
Change TOKEN_SWAP_PROGRAM_ID in js/src/index.ts

```sh
npm start
```

## Working with Website


```sh
cd web-ui
```
```sh
yarn install
```
Change PROGRAM_IDS in web-ui/src/utils/ids.tsx as the id of AMM Cropper
```sh
yarn start
```











