import Head from "next/head";
import { NextPage } from "next";

import MySteak from "components/MySteak";
import UnbondQueue from "components/UnbondQueue";

const IndexPage: NextPage = () => {
  return (
    <>
      <Head>
        <title>Steak | My Steak</title>
      </Head>
      <MySteak />
      <UnbondQueue />
    </>
  );
};

export default IndexPage;
