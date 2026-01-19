import { ref } from 'vue'

export function useEntitySelection() {
    const showEntityModal = ref(false)
    const previewEntities = ref<any[]>([])
    const previewByType = ref<Record<string, any[]>>({})
    const selectedEntityIds = ref<string[] | null>(null) // null = use all entities
    const expectedTotal = ref<number | null>(null)

    return {
        showEntityModal,
        previewEntities,
        previewByType,
        selectedEntityIds,
        expectedTotal
    }
}
