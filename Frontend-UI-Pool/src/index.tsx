import React from "react";
import ReactDOM from "react-dom";
import "./index.css";
import App from "./App";
import * as serviceWorker from "./serviceWorker";
import { WalletProvider } from "./utils/wallet";
import { ConnectionProvider } from "./utils/connection";
import { AccountsProvider } from "./utils/accounts";
import { CurrencyPairProvider } from "./utils/currencyPair";

import { CssBaseline } from "@material-ui/core";
import { ThemeProvider } from "@material-ui/core/styles";
import { SnackbarProvider } from "notistack";
import { Provider } from "react-redux";
import { HashRouter } from "react-router-dom";
import BackgroundImage from "./components/BackgroundImage";
import { EthereumProviderProvider } from "./contexts/EthereumProviderContext";
import { SolanaWalletProvider } from "./contexts/SolanaWalletContext";
import { TerraWalletProvider } from "./contexts/TerraWalletContext";
import ErrorBoundary from "./ErrorBoundary";
import { theme } from "./muiTheme";
import { store } from "./store";


ReactDOM.render(
  <React.StrictMode>
    <Provider store={store}>
      <ThemeProvider theme={theme}>
        <CssBaseline />
        <ErrorBoundary>
          <SnackbarProvider maxSnack={3}>
            <SolanaWalletProvider>
            <EthereumProviderProvider>
            <TerraWalletProvider>
            <HashRouter>
                    <ConnectionProvider>
                      <WalletProvider>
                        <AccountsProvider>
                          <CurrencyPairProvider>
                            <App />
                          </CurrencyPairProvider>
                        </AccountsProvider>
                      </WalletProvider>
                    </ConnectionProvider>
            </HashRouter>
            </TerraWalletProvider>
            </EthereumProviderProvider>
            </SolanaWalletProvider>
          </SnackbarProvider>
        </ErrorBoundary>
      </ThemeProvider>
    </Provider>
  </React.StrictMode>,
  document.getElementById("root")
);

// If you want your app to work offline and load faster, you can change
// unregister() to register() below. Note this comes with some pitfalls.
// Learn more about service workers: https://bit.ly/CRA-PWA
serviceWorker.unregister();
