import React from 'react'
import {
  CHAIN_ID_SOLANA,
  CHAIN_ID_TERRA,
  getForeignAssetEth,
  getForeignAssetSolana,
  getForeignAssetTerra,
  hexToNativeString,
  hexToUint8Array,
} from "@certusone/wormhole-sdk";
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
import {
  getEvmChainId,
  getNFTBridgeAddressForChain,
  getTokenBridgeAddressForChain,
  SOLANA_HOST,
  SOL_NFT_BRIDGE_ADDRESS,
  SOL_TOKEN_BRIDGE_ADDRESS,
  TERRA_HOST,
  TERRA_TOKEN_BRIDGE_ADDRESS,
} from "../utils/consts";
import { isEVMChain } from "../utils/ethereum";

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
  const { provider, chainId: evmChainId } = useEthereumProvider();
  const correctEvmNetwork = getEvmChainId(targetChain);
  const hasCorrectEvmNetwork = evmChainId === correctEvmNetwork;
  useEffect(() => {
    if (isSourceAssetWormholeWrapped && originChain === targetChain) {
      dispatch(setTargetAsset(hexToNativeString(originAsset, originChain)));
      return;
    }
    // TODO: loading state, error state
    let cancelled = false;
    (async () => {
      if (
        isEVMChain(targetChain) &&
        provider &&
        hasCorrectEvmNetwork &&
        originChain &&
        originAsset
      ) {
        dispatch(setTargetAsset(undefined));
        try {
          const asset = await (nft
            ? getForeignAssetEthNFT(
                getNFTBridgeAddressForChain(targetChain),
                provider,
                originChain,
                hexToUint8Array(originAsset)
              )
            : getForeignAssetEth(
                getTokenBridgeAddressForChain(targetChain),
                provider,
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
      if (targetChain === CHAIN_ID_SOLANA && originChain && originAsset) {
        dispatch(setTargetAsset(undefined));
        try {
          const connection = new Connection(SOLANA_HOST, "confirmed");
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
        dispatch(setTargetAsset(undefined));
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
    hasCorrectEvmNetwork,
  ]);
}

export default useFetchTargetAsset;