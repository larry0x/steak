import { Icon, IconProps } from "@chakra-ui/react";

export default function SuccessIcon(props: IconProps) {
  return (
    <Icon viewBox="0 0 80 80" fill="none" {...props}>
      <circle
        cx="40"
        cy="40"
        r="38.5"
        transform="rotate(180 40 40)"
        stroke="#15BFA9"
        fill="none"
        strokeWidth="3"
      />
      <path
        d="M22 43.5L34.507 54L59 27"
        stroke="#15BFA9"
        fill="none"
        strokeWidth="3"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </Icon>
  );
}
