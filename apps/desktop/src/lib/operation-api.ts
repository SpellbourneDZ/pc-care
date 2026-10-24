import { Channel, invoke } from '@tauri-apps/api/core'
import type { ApiReply } from '@/generated/ApiReply'
import type { ApiRequest } from '@/generated/ApiRequest'
import type { TaskProgress } from '@/generated/TaskProgress'

export async function operationApi(
  request: ApiRequest,
  onProgress?: (progress: TaskProgress) => void,
): Promise<ApiReply> {
  let timer: ReturnType<typeof setTimeout> | undefined
  try {
    const progress = new Channel<TaskProgress>(onProgress)
    return await Promise.race([
      invoke<ApiReply>('operation_api', { request, progress }),
      new Promise<ApiReply>((resolve) => {
        timer = setTimeout(
          () => resolve({ status: 'error', error: 'timeout' }),
          5_000,
        )
      }),
    ])
  } catch {
    return { status: 'error', error: 'transport' }
  } finally {
    clearTimeout(timer)
  }
}
