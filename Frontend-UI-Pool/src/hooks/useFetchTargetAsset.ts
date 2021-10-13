import {
  CHAIN_ID_ETH,
  CHAIN_ID_SOLANA,
  CHAIN_ID_TERRA,
  getForeignAssetEth,
  getForeignAssetSolana,
  getForeignAssetTerra,
} from "@certusone/wormhole-sdk";
import React from 'react'
import {
  getForeignAssetEth as getForeignAssetEthNFT,
  getForeignAssetSol as getForeignAssetSolNFT,
} from "@certusone/wormhole-sdk/lib/nft_bridge";
import { BigNumber } from "@ethersproject/bignumber";
import { arrayify } from "@ethersproject/bytes";
import { Connection } from "@solana/web3.js";
import { LCDClient } from "@terra-money/terra.js";
import { useEffect } from "react";
import { useDispatch, useSelector } from "react-redux";
import { useEthereumProvider } from "../contexts/EthereumProviderContext";
import { setTargetAsset as setNFTTargetAsset } from "../store/nftSlice";
import {
  selectNFTIsSourceAssetWormholeWrapped,
  selectNFTOriginAsset,
  selectNFTOriginChain,
  selectNFTOriginTokenId,
  selectNFTTargetChain,
  selectTransferIsSourceAssetWormholeWrapped,
  selectTransferOriginAsset,
  selectTransferOriginChain,
  selectTransferTargetChain,
} from "../store/selectors";
import { setTargetAsset as setTransferTargetAsset } from "../store/transferSlice";
import { hexToNativeString, hexToUint8Array } from "../utils/array";
import {
  ETH_NFT_BRIDGE_ADDRESS,
  ETH_TOKEN_BRIDGE_ADDRESS,
  SOLANA_HOST,
  SOL_NFT_BRIDGE_ADDRESS,
  SOL_TOKEN_BRIDGE_ADDRESS,
  TERRA_HOST,
  TERRA_TOKEN_BRIDGE_ADDRESS,
} from "../utils/consts";

function useFetchTargetAsset(nft?: boolean) {
  const dispatch = useDispatch();
  const isSourceAssetWormholeWrapped = useSelector(
    nft
      ? selectNFTIsSourceAssetWormholeWrapped
      : selectTransferIsSourceAssetWormholeWrapped
  );
  const originChain = useSelector(
    nft ? selectNFTOriginChain : selectTransferOriginChain
  );
  const originAsset = useSelector(
    nft ? selectNFTOriginAsset : selectTransferOriginAsset
  );
  const originTokenId = useSelector(selectNFTOriginTokenId);
  const tokenId = originTokenId || ""; // this should exist by this step for NFT transfers
  const targetChain = useSelector(
    nft ? selectNFTTargetChain : selectTransferTargetChain
  );

  const setTargetAsset = nft ? setNFTTargetAsset : setTransferTargetAsset;
  const { provider } = useEthereumProvider();
  useEffect(() => {
    console.log('useEffect')
    if (isSourceAssetWormholeWrapped && originChain === targetChain) {
      console.log('isSourceAssetWormholeWrapped', isSourceAssetWormholeWrapped)
      console.log('originChain', originChain)
      console.log('targetChain', targetChain)
      console.log('originAsset', originAsset)
      console.log('hexToNativeString(originAsset, originChain)', hexToNativeString(originAsset, originChain))
      dispatch(setTargetAsset(hexToNativeString(originAsset, originChain)));
      return;
    }
    console.log('setting undefined')
    // TODO: loading state, error state
    dispatch(setTargetAsset(undefined));
    console.log('set undefined')

    console.log('CHAIN_ID_ETH', CHAIN_ID_ETH)
    console.log('provider', provider)
    console.log('originChain', originChain)
    console.log('originAsset', originAsset)
    console.log('targetChain', targetChain)
    console.log('CHAIN_ID_SOLANA', CHAIN_ID_SOLANA)

    let cancelled = false;
    (async () => {
      if (
        targetChain === CHAIN_ID_ETH &&
        provider &&
        originChain &&
        originAsset
      ) {
        console.log('targetChain is CHAIN_ID_ETH')
        try {
          const asset = await (nft
            ? getForeignAssetEthNFT(
                ETH_NFT_BRIDGE_ADDRESS,
                provider,
                originChain,
                hexToUint8Array(originAsset)
              )
            : getForeignAssetEth(
                ETH_TOKEN_BRIDGE_ADDRESS,
                provider,
                originChain,
                hexToUint8Array(originAsset)
              ));
          if (!cancelled) {
            console.log('setting asset', asset)
            dispatch(setTargetAsset(asset));
          }
        } catch (e) {
          if (!cancelled) {
            // TODO: warning for this
            dispatch(setTargetAsset(null));
          }
        }
      }
      if (targetChain === CHAIN_ID_SOLANA && originChain && originAsset) {
        console.log('targetChain is CHAIN_ID_SOLANA')
        try {
          const connection = new Connection(SOLANA_HOST, "confirmed");
          console.log('SOLANA_HOST', SOLANA_HOST)
          console.log("Getting origin asset", connection)
          const asset = await (nft
            ? getForeignAssetSolNFT(
                SOL_NFT_BRIDGE_ADDRESS,
                originChain,
                hexToUint8Array(originAsset),
                arrayify(BigNumber.from(tokenId || "0"))
              )
            : getForeignAssetSolana(
                connection,
                SOL_TOKEN_BRIDGE_ADDRESS,
                originChain,
                hexToUint8Array(originAsset)
              ));
          if (!cancelled) {
            dispatch(setTargetAsset(asset));
          }
        } catch (e) {
          if (!cancelled) {
            // TODO: warning for this
            dispatch(setTargetAsset(null));
          }
        }
      }
      if (targetChain === CHAIN_ID_TERRA && originChain && originAsset) {
        console.log('targetChain = TERRA')
        try {
          const lcd = new LCDClient(TERRA_HOST);
          const asset = await getForeignAssetTerra(
            TERRA_TOKEN_BRIDGE_ADDRESS,
            lcd,
            originChain,
            hexToUint8Array(originAsset)
          );
          if (!cancelled) {
            dispatch(setTargetAsset(asset));
          }
        } catch (e) {
          if (!cancelled) {
            // TODO: warning for this
            dispatch(setTargetAsset(null));
          }
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [
    dispatch,
    isSourceAssetWormholeWrapped,
    originChain,
    originAsset,
    targetChain,
    provider,
    nft,
    setTargetAsset,
    tokenId,
  ]);
}

export default useFetchTargetAsset;
