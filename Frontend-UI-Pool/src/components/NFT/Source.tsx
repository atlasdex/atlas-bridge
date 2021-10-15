import * as React from 'react';
import { Button, makeStyles, MenuItem, TextField } from "@material-ui/core";
import { Restore, VerifiedUser } from "@material-ui/icons";
import { Alert } from "@material-ui/lab";
import { useCallback } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Link } from "react-router-dom";
import useIsWalletReady from "../../hooks/useIsWalletReady";
import { incrementStep, setSourceChain } from "../../store/nftSlice";
import {
  selectNFTIsSourceComplete,
  selectNFTShouldLockFields,
  selectNFTSourceBalanceString,
  selectNFTSourceChain,
  selectNFTSourceError,
} from "../../store/selectors";
import { CHAINS_WITH_NFT_SUPPORT } from "../../utils/consts";
import { isEVMChain } from "../../utils/ethereum";
import ButtonWithLoader from "../ButtonWithLoader";
import KeyAndBalance from "../KeyAndBalance";
import LowBalanceWarning from "../LowBalanceWarning";
import StepDescription from "../StepDescription";
import { TokenSelector } from "../TokenSelectors/SourceTokenSelector";

const useStyles = makeStyles((theme) => ({
  transferField: {
    marginTop: theme.spacing(5),
  },
  buttonWrapper: {
    textAlign: "right",
  },
  nftOriginVerifierButton: {
    marginTop: theme.spacing(0.5),
  },
}));

function Source({
  setIsRecoveryOpen,
}: {
  setIsRecoveryOpen: (open: boolean) => void;
}) {
  const classes = useStyles();
  const dispatch = useDispatch();
  const sourceChain = useSelector(selectNFTSourceChain);
  const uiAmountString = useSelector(selectNFTSourceBalanceString);
  const error = useSelector(selectNFTSourceError);
  const isSourceComplete = useSelector(selectNFTIsSourceComplete);
  const shouldLockFields = useSelector(selectNFTShouldLockFields);
  const { isReady, statusMessage } = useIsWalletReady(sourceChain);
  const handleSourceChange = useCallback(
    (event) => {
      dispatch(setSourceChain(event.target.value));
    },
    [dispatch]
  );
  const handleNextClick = useCallback(() => {
    dispatch(incrementStep());
  }, [dispatch]);
  return (
    <>
      <StepDescription>
        <div style={{ display: "flex", alignItems: "center" }}>
          Select an NFT to send through the Wormhole NFT Bridge.
          <div style={{ flexGrow: 1 }} />
          <div>
            <div className={classes.buttonWrapper}>
              <Button
                onClick={() => setIsRecoveryOpen(true)}
                size="small"
                variant="outlined"
                endIcon={<Restore />}
              >
                Perform Recovery
              </Button>
            </div>
            <div className={classes.buttonWrapper}>
              <Button
                component={Link}
                to="/nft-origin-verifier"
                size="small"
                variant="outlined"
                endIcon={<VerifiedUser />}
                className={classes.nftOriginVerifierButton}
              >
                NFT Origin Verifier
              </Button>
            </div>
          </div>
        </div>
      </StepDescription>
      <TextField
        select
        fullWidth
        value={sourceChain}
        onChange={handleSourceChange}
        disabled={shouldLockFields}
      >
        {CHAINS_WITH_NFT_SUPPORT.map(({ id, name }) => (
          <MenuItem key={id} value={id}>
            {name}
          </MenuItem>
        ))}
      </TextField>
      {isEVMChain(sourceChain) ? (
        <Alert severity="info">
          Only NFTs which implement ERC-721 are supported.
        </Alert>
      ) : null}
      <KeyAndBalance chainId={sourceChain} balance={uiAmountString} />
      {isReady || uiAmountString ? (
        <div className={classes.transferField}>
          <TokenSelector disabled={shouldLockFields} nft={true} />
        </div>
      ) : null}
      <LowBalanceWarning chainId={sourceChain} />
      <ButtonWithLoader
        disabled={!isSourceComplete}
        onClick={handleNextClick}
        showLoader={false}
        error={statusMessage || error}
      >
        Next
      </ButtonWithLoader>
    </>
  );
}

export default Source;