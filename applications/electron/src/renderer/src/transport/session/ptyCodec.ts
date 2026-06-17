export const encodeKeystroke = (data: string): string => {
  const bytes = new TextEncoder().encode(data)
  let binary = ''
  for (const b of bytes) binary += String.fromCharCode(b)
  return btoa(binary)
}

export const decodePtyData = (b64: string): Uint8Array => {
  const binary = atob(b64)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}
