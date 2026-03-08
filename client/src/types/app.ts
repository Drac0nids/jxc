export interface UserSession {
  accessToken: string
  refreshToken: string
  expiresIn: number
  tenantId: string
  user: {
    id: string
    name: string
    role: string
  }
}

export interface AppMessage {
  type: 'success' | 'error' | 'info'
  text: string
}
