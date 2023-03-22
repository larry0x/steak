const styles = {
  baseStyle: {
    overlay: {
      backdropFilter: "blur(12px)",
    },
    dialog: {
      borderRadius: "2xl",
      bg: "brand.lightBrown",
      p: "6",
      boxShadow: "2xl",
    },
    header: {
      flex: 1,
      px: "0",
    },
    closeButton: {
      position: "static",
      p: "3",
      borderWidth: "1px",
      borderColor: "brand.black",
      borderRadius: "full",
      _focus: {
        boxShadow: "none",
      },
    },
  },
};

export default styles;
