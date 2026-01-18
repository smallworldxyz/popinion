import { ref } from 'vue';

const loading = ref(false);
const loadingMessage = ref('Loading...');

export const eventBus = {
  showLoading: (message = 'Loading...') => {
    loadingMessage.value = message;
    loading.value = true;
  },
  hideLoading: () => {
    loading.value = false;
  },
  loading,
  loadingMessage,
};
