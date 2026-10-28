import { afterEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { toast } from 'vue-sonner'
import OperationFeedback from '../src/components/OperationFeedback.vue'

afterEach(() => { vi.restoreAllMocks() })

describe('unified operation notifications', () => {
  it('keeps errors until dismissed and clears the old error on retry', async () => {
    const error = vi.spyOn(toast, 'error').mockReturnValue('error-1')
    const dismiss = vi.spyOn(toast, 'dismiss').mockReturnValue('error-1')
    const wrapper = mount(OperationFeedback, { props: { error: '保存失败，草稿保留' } })
    expect(error).toHaveBeenCalledWith('保存失败，草稿保留', { duration: Infinity })
    expect(wrapper.text()).toBe('')
    await wrapper.setProps({ error: '' })
    expect(dismiss).toHaveBeenCalledWith('error-1')
    await wrapper.setProps({ error: '保存失败，草稿保留' })
    expect(error).toHaveBeenCalledTimes(2)
    wrapper.unmount()
  })

  it('shows success with a finite duration and wires manual dismissal', () => {
    const success = vi.spyOn(toast, 'success').mockReturnValue('success-1')
    const wrapper = mount(OperationFeedback, { props: { message: '已保存' } })
    expect(success).toHaveBeenCalledWith('已保存', expect.objectContaining({ duration: 5000 }))
    success.mock.calls[0]![1]!.onDismiss!({ id: 'success-1', title: '已保存' })
    expect(wrapper.emitted('dismiss')).toHaveLength(1)
    wrapper.unmount()
  })

  it('uses the same notification system for warnings and information', () => {
    const warning = vi.spyOn(toast, 'warning').mockReturnValue('warning-1')
    const info = vi.spyOn(toast, 'info').mockReturnValue('info-1')
    const first = mount(OperationFeedback, { props: { message: '请核对账号', variant: 'warning' } })
    const second = mount(OperationFeedback, { props: { message: '已打开浏览器', variant: 'info' } })
    expect(warning).toHaveBeenCalledWith('请核对账号', expect.objectContaining({ duration: 8000 }))
    expect(info).toHaveBeenCalledWith('已打开浏览器', expect.objectContaining({ duration: 5000 }))
    first.unmount(); second.unmount()
  })
})
