import { extendTheme } from "@chakra-ui/react";

import Button from "./button";
import Link from "./link";
import Popover from "./popover";
import Modal from "./modal";
import Text from "./text";

const defaultSansSerif = "-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Helvetica Neue,Arial,Noto Sans,sans-serif";
const defaultEmoji = "Apple Color Emoji,Segoe UI Emoji,Segoe UI Symbol,Noto Color Emoji";

export default extendTheme({
  fonts: {
    heading: `Urbanist,${defaultSansSerif},${defaultEmoji}`,
    body: `Urbanist,${defaultSansSerif},${defaultEmoji}`,
    mono: "Menlo, monospace",
  },
  components: {
    Button,
    Link,
    Popover,
    Modal,
    Text,
  },
  colors: {
    brand: {
      darkerBrown: "#a08b77",
      darkBrown: "#d2bba6",
      lightBrown: "#f5d9c0",
      lighterBrown: "#e4d5c8",
      red: "#d9474b",
      black: "#312b26",
    },
  },
  textStyles: {
    minibutton: {
      fontWeight: "bolder",
      fontSize: "12px",
      lineHeight: "1.2",
      letterSpacing: "0.18rem",
      textTransform: "uppercase",
    },
    small: {
      fontWeight: "medium",
      fontSize: "sm",
      lineHeight: "shorter",
    },
  },
});
