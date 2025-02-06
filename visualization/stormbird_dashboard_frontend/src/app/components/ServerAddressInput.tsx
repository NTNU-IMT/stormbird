import React, { useState, useEffect } from 'react';

interface ServerAddressInputProps {
  onAddressChange: (address: string) => void;
  defaultAddress?: string;
}

const ServerAddressInput: React.FC<ServerAddressInputProps> = ({ onAddressChange, defaultAddress = '' }) => {
  const [address, setAddress] = useState(defaultAddress);

  useEffect(() => {
    onAddressChange(defaultAddress);
  }, [defaultAddress, onAddressChange]);

  const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const newAddress = event.target.value;
    setAddress(newAddress);
    onAddressChange(newAddress);
  };

  return (
    <div>
      <label htmlFor="server-address">Server Address:</label>
      <input
        type="text"
        id="server-address"
        value={address}
        onChange={handleChange}
      />
    </div>
  );
};

export default ServerAddressInput;