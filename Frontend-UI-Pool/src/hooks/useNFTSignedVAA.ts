import React from 'react'
import { useMemo } from "react";
import { useSelector } from "react-redux";
import { selectNFTSignedVAAHex } from "../store/selectors";
import { hexToUint8Array } from "../utils/array";

export default function useNFTSignedVAA() {
  const signedVAAHex = useSelector(selectNFTSignedVAAHex);
  const signedVAA = useMemo(
    () => (signedVAAHex ? hexToUint8Array(signedVAAHex) : undefined),
    [signedVAAHex]
  );
  return signedVAA;
}
