import { HashRouter, Route } from "react-router-dom";
import React from "react";
import { ExchangeView } from "./components/exchange";

export function Routes() {
  // TODO: add simple view for sharing ...
  return (
    <>
      <HashRouter basename={"/"}>
<<<<<<< HEAD
        <ConnectionProvider>
          <WalletProvider>
            <AccountsProvider>
              <MarketProvider>
                <CurrencyPairProvider>
                  <Route exact path="/" component={ExchangeView} />
                  <Route exact path="/add" component={ExchangeView} />
                  <Route exact path="/bridge" component={ExchangeView} />
                  <Route exact path="/info" component={() => <ChartsView />} />
                  <Route
                    exact
                    path="/pool"
                    component={() => <PoolOverview />}
                  />
                </CurrencyPairProvider>
              </MarketProvider>
            </AccountsProvider>
          </WalletProvider>
        </ConnectionProvider>
=======
        <Route exact path="/" component={ExchangeView} />
>>>>>>> c148b1ea1aef1f9160d4d65c7c67fe2abecc2001
      </HashRouter>
    </>
  );
}

