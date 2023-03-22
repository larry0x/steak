import { Icon, IconProps } from "@chakra-ui/react";

export default function CloseIcon(props: IconProps) {
  return (
    <Icon viewBox="0 0 24 24" fill="#000" {...props}>
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M0 12C0 5.37258 5.37258 0 12 0V0C18.6274 0 24 5.37258 24 12V12C24 18.6274 18.6274 24 12 24V24C5.37258 24 0 18.6274 0 12V12Z M0.500001 12C0.500002 5.64873 5.64873 0.500002 12 0.500003C18.3513 0.500003 23.5 5.64873 23.5 12C23.5 18.3513 18.3513 23.5 12 23.5C5.64873 23.5 0.5 18.3513 0.500001 12Z"
      />
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M12.9424 12.001L16.7144 8.22906L15.7716 7.28625L11.9996 11.0582L8.22804 7.28661L7.28523 8.22942L11.0568 12.001L7.28632 15.7715L8.22913 16.7143L11.9996 12.9438L15.7705 16.7147L16.7133 15.7719L12.9424 12.001Z"
      />
    </Icon>
  );
}
