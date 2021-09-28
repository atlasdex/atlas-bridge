import React from "react";
import { Button, Card, Popover } from "antd";
import { TradeEntry } from "./trade";
import { AddToLiquidity } from "./pool/add";
import { Transfer } from "./Transfer";
import { Settings } from "./settings";
import { SettingOutlined } from "@ant-design/icons";
import { AppBar } from "./appBar";
import { useHistory, useLocation } from "react-router-dom";

export const ExchangeView = (props: {}) => {
  const tabStyle: React.CSSProperties = { width: 120 };
  const tabList = [
    {
      key: "trade",
      tab: <div style={tabStyle}>Swap</div>,
      render: () => {
        return <TradeEntry />;
      },
    },
    {
      key: "pool",
      tab: <div style={tabStyle}>Pool</div>,
      render: () => {
        return <AddToLiquidity />;
      },
    },
    {
        key: "bridge",
        tab: <div style={tabStyle}>Bridge</div>,
        render: () => {
          return <Transfer />;
        },
    },
  ];

  const location = useLocation();
  const history = useHistory();


  const getActiveTab = () => {
    if(location.pathname.indexOf("add") >= 0)
      return "pool";
    else if (location.pathname.indexOf("bridge") >= 0)
      return "bridge";
    else
      return "trade";
  }
  const activeTab = getActiveTab();

  //const activeTab = location.pathname.indexOf("add") < 0 ? "trade" : "pool";

  const handleTabChange = (key: any) => {
    if (activeTab !== key) {
      if (key === "trade") {
        history.push("/");
      } else if(key === "pool") {
        history.push("/add");
      } else {
        history.push("/bridge")
      }
    }
  };

  return (
    <>
      <AppBar
        right={
          <Popover
            placement="topRight"
            title="Settings"
            content={<Settings />}
            trigger="click"
          >
            <Button
              shape="circle"
              size="large"
              type="text"
              icon={<SettingOutlined />}
            />
          </Popover>
        }
      />
      <Card
        className="exchange-card"
        headStyle={{ padding: 0 }}
        bodyStyle={{ position: "relative" }}
        tabList={tabList}
        tabProps={{
          tabBarGutter: 0,
        }}
        activeTabKey={activeTab}
        onTabChange={(key) => {
          handleTabChange(key);
        }}
      >
        {tabList.find((t) => t.key === activeTab)?.render()}
      </Card>
    </>
  );
};
