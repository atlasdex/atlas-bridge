import * as React from 'react';
import { makeStyles, Typography } from "@material-ui/core";
import { useSelector } from "react-redux";
import {
  selectTransferTargetAddressHex,
  selectTransferTargetChain,
} from "../../store/selectors";
import { hexToNativeString } from "../../utils/array";
import { CHAINS_BY_ID } from "../../utils/consts";
import SmartAddress from "../SmartAddress";

const useStyles = makeStyles((theme) => ({
  description: {
    textAlign: "center",
  },
}));

export default function TargetPreview() {
  const classes = useStyles();
  const targetChain = useSelector(selectTransferTargetChain);
  const targetAddress = useSelector(selectTransferTargetAddressHex);
  const targetAddressNative = hexToNativeString(targetAddress, targetChain);

  const explainerContent =
    targetChain && targetAddressNative ? (
      <>
        <span>to</span>
        <SmartAddress chainId={targetChain} address={targetAddressNative} />
        <span>on {CHAINS_BY_ID[targetChain].name}</span>
      </>
    ) : (
      ""
    );

  return (
    <Typography
      component="div"
      variant="subtitle2"
      className={classes.description}
    >
      {explainerContent}
    </Typography>
  );
}
