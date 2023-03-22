const button = {
  variants: {
    primary: {
      outline: "none",
      borderColor: "brand.red",
      borderWidth: "2px",
      borderRadius: "full",
      bg: "brand.red",
      color: "white",
      px: "10",
      py: "2",
      _hover: {
        bg: "transparent",
        color: "brand.red",
      },
    },
    mini: {
      outline: "none",
      borderRadius: "full",
      color: "white",
      bg: "brand.red",
      px: "2",
      h: "auto",
      py: "0.5",
      fontSize: "11px",
      fontWeight: "800",
      border: "2px solid #d9474b",
      letterSpacing: "widest",
      textTransform: "uppercase",
      _hover: {
        bg: "white",
        color: "brand.red",
      },
    },
    simple: {
      outline: "none",
      borderRadius: "none",
      bg: "none",
      px: "none",
      _hover: {
        bg: "none",
      },
    },
  },
};

export default button;
