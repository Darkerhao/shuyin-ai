import { ref } from 'vue'
import { workflowSnapshot, type WorkflowSnapshot } from '../services/workflow'
const snapshot = ref<WorkflowSnapshot>({ resume: [], running: [] })
const error = ref('')
let pending: Promise<void> | undefined
async function refresh() {
  if (pending) return pending
  pending = workflowSnapshot().then(value => { snapshot.value = value; error.value = '' }, () => { error.value = '任务状态暂时无法同步，当前显示上次状态。' }).finally(() => { pending = undefined })
  return pending
}
export const useWorkflow = () => ({ snapshot, error, refresh })
