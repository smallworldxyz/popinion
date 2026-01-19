<template>
  <div class="interaction-panel" @click="clearSelection">
    <!-- Selection Popup for Knowledge Pad -->
    <Teleport to="body">
      <div 
        v-if="selectionPopup && !showTagModal" 
        class="selection-popup"
        :style="{ left: selectionPopup.x + 'px', top: selectionPopup.y + 'px' }"
        @click.stop
      >
        <button class="add-knowledge-btn" @click="openTagModal">
          📋 Add to Knowledge
        </button>
      </div>
    </Teleport>

    <!-- Tag Modal for Knowledge Capture -->
    <Teleport to="body">
      <div v-if="showTagModal" class="tag-modal-overlay" @click.self="closeTagModal">
        <div class="tag-modal" @click.stop>
          <div class="tag-modal-header">
            <h3>📋 Add to Knowledge Pad</h3>
            <button class="tag-modal-close" @click="closeTagModal">×</button>
          </div>
          <div class="tag-modal-body">
            <div class="tag-preview">
              <p class="preview-text">"{{ pendingHighlight?.text?.substring(0, 150) }}{{ pendingHighlight?.text?.length > 150 ? '...' : '' }}"</p>
              <p class="preview-source">From: {{ cleanAgentName(pendingHighlight?.source?.agent) }}</p>
            </div>
            
            <div class="tag-section">
              <label class="tag-label">Select Tags:</label>
              <div class="predefined-tags">
                <button 
                  v-for="tag in predefinedTags" 
                  :key="tag"
                  class="tag-chip"
                  :class="{ selected: selectedTags.has(tag) }"
                  @click="toggleTag(tag)"
                >{{ tag }}</button>
              </div>
            </div>
            
            <div class="custom-tag-section">
              <label class="tag-label">Add Custom Tag:</label>
              <div class="custom-tag-input">
                <input 
                  v-model="customTagInput" 
                  placeholder="Type and press Enter"
                  @keydown.enter.prevent="addCustomTag"
                />
                <button class="add-tag-btn" @click="addCustomTag">+</button>
              </div>
            </div>
            
            <div v-if="selectedTags.size > 0" class="selected-tags-preview">
              <span class="tags-label">Selected:</span>
              <span v-for="tag in selectedTags" :key="tag" class="selected-tag">
                {{ tag }}
                <button @click="toggleTag(tag)">×</button>
              </span>
            </div>
          </div>
          <div class="tag-modal-footer">
            <button class="tag-btn secondary" @click="closeTagModal">Cancel</button>
            <button class="tag-btn primary" @click="confirmAddToKnowledge">
              Add to Knowledge Pad
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Injection Removal Modal -->
    <Teleport to="body">
      <div v-if="showRemovalModal" class="removal-modal-overlay" @click.self="closeRemovalModal">
        <div class="removal-modal" @click.stop>
          <div class="removal-modal-header">
            <h3>🧠 Injected Knowledge</h3>
            <button class="removal-modal-close" @click="closeRemovalModal">×</button>
          </div>
          <div class="removal-modal-body">
            <div v-if="removalModalItems.length === 0" class="removal-empty">
              <p>No injected knowledge for this target.</p>
            </div>
            <div v-else class="removal-list">
              <div 
                v-for="(item, idx) in removalModalItems" 
                :key="idx"
                class="removal-item"
              >
                <p class="removal-text">"{{ item.length > 100 ? item.substring(0, 100) + '...' : item }}"</p>
                <button 
                  class="removal-btn" 
                  @click="removeInjection(removalModalTarget, idx)"
                  title="Remove this injection"
                >🗑️</button>
              </div>
            </div>
          </div>
          <div class="removal-modal-footer">
            <button 
              v-if="removalModalItems.length > 0"
              class="removal-clear-btn"
              @click="clearAllInjections(removalModalTarget)"
            >Clear All ({{ removalModalItems.length }})</button>
            <button class="removal-done-btn" @click="closeRemovalModal">Done</button>
          </div>
        </div>
      </div>
    </Teleport>
    <!-- Main Split Layout -->
    <div class="main-split-layout" :class="{ 'agora-mode': activeTab === 'agora' && agoraDebateActive }">
      <!-- LEFT PANEL: Report Style -->
      <div class="left-panel report-style" :class="{ 'hidden': activeTab === 'agora' && agoraDebateActive }" ref="leftPanel">
        <div v-if="reportOutline" class="report-content-wrapper">
          <!-- Report Header -->
          <div class="report-header-block">
            <div class="report-meta">
              <span class="report-tag">Prediction Report</span>
              <span class="report-id">ID: {{ reportId || 'REF-2024-X92' }}</span>
            </div>
            <h1 class="main-title">{{ reportOutline.title }}</h1>
            <p class="sub-title">{{ reportOutline.summary }}</p>
            <div class="header-divider"></div>
          </div>

          <!-- Sections List -->
          <div class="sections-list">
            <div 
              v-for="(section, idx) in reportOutline.sections" 
              :key="idx"
              class="report-section-item"
              :class="{ 
                'is-active': currentSectionIndex === idx + 1,
                'is-completed': isSectionCompleted(idx + 1),
                'is-pending': !isSectionCompleted(idx + 1) && currentSectionIndex !== idx + 1
              }"
            >
              <div class="section-header-row" @click="toggleSectionCollapse(idx)" :class="{ 'clickable': isSectionCompleted(idx + 1) }">
                <h3 class="section-title">{{ section.title }}</h3>
                <svg 
                  v-if="isSectionCompleted(idx + 1)" 
                  class="collapse-icon" 
                  :class="{ 'is-collapsed': collapsedSections.has(idx) }"
                  viewBox="0 0 24 24" 
                  width="20" 
                  height="20" 
                  fill="none" 
                  stroke="currentColor" 
                  stroke-width="2"
                >
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
              </div>
              
              <div class="section-body" v-show="!collapsedSections.has(idx)">
                <!-- Completed Content -->
                <div v-if="generatedSections[idx + 1]" class="generated-content" v-html="renderMarkdown(generatedSections[idx + 1])"></div>
                
                <!-- Loading State -->
                <div v-else-if="currentSectionIndex === idx + 1" class="loading-state">
                  <div class="loading-icon">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                      <circle cx="12" cy="12" r="10" stroke-width="4" stroke="#E5E7EB"></circle>
                      <path d="M12 2a10 10 0 0 1 10 10" stroke-width="4" stroke="#4B5563" stroke-linecap="round"></path>
                    </svg>
                  </div>
                  <span class="loading-text">Generating{{ section.title }}...</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Waiting State -->
        <div v-if="!reportOutline" class="waiting-placeholder">
          <div class="waiting-animation">
            <div class="waiting-ring"></div>
            <div class="waiting-ring"></div>
            <div class="waiting-ring"></div>
          </div>
          <span class="waiting-text">Waiting for Report Agent...</span>
        </div>
      </div>

      <!-- RIGHT PANEL: Interaction Interface -->
      <div class="right-panel" ref="rightPanel">
        <!-- Unified Action Bar - Professional Design -->
        <div class="action-bar">
        <div class="action-bar-header">
          <svg class="action-bar-icon" viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
          </svg>
          <div class="action-bar-text">
            <span class="action-bar-title">Interactive Tools</span>
            <span class="action-bar-subtitle mono">{{ profiles.length }} agents available</span>
          </div>
        </div>
          <div class="action-bar-tabs">
            <button 
              class="tab-pill"
              :class="{ active: activeTab === 'chat' && chatTarget === 'report_agent' }"
              @click="selectReportAgentChat"
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"></path>
              </svg>
              <span>Chat with Report Agent</span>
            </button>
            <div class="agent-dropdown" v-if="profiles.length > 0">
              <button 
                class="tab-pill agent-pill"
                :class="{ active: activeTab === 'chat' && chatTarget === 'agent' }"
                @click="toggleAgentDropdown"
              >
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
                  <circle cx="12" cy="7" r="4"></circle>
                </svg>
                <span>{{ selectedAgent ? selectedAgent.username : 'Chat with any agent in the world' }}</span>
                <svg class="dropdown-arrow" :class="{ open: showAgentDropdown }" viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
              </button>
              <div v-if="showAgentDropdown" class="dropdown-menu">
                <div class="dropdown-header">Select Chat Target</div>
                <div 
                  v-for="(agent, idx) in profiles" 
                  :key="idx"
                  class="dropdown-item"
                  @click="selectAgent(agent, idx)"
                >
                  <div class="agent-avatar">{{ (agent.username || 'A')[0] }}</div>
                  <div class="agent-info">
                    <span class="agent-name">{{ agent.username }}</span>
                    <span class="agent-role">{{ agent.profession || 'Unknown Profession' }}</span>
                  </div>
                </div>
              </div>
            </div>
            <div class="tab-divider"></div>
            <button 
              class="tab-pill survey-pill"
              :class="{ active: activeTab === 'panel' }"
              @click="selectPanelTab"
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
                <circle cx="9" cy="7" r="4"></circle>
                <path d="M23 21v-2a4 4 0 0 0-3-3.87"></path>
                <path d="M16 3.13a4 4 0 0 1 0 7.75"></path>
              </svg>
              <span>Panel Discussion</span>
            </button>
            <button 
              class="tab-pill agora-pill"
              :class="{ active: activeTab === 'agora' }"
              @click="selectAgoraTab"
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 21h18M5 21V7l7-4 7 4v14M9 21v-6h6v6"></path>
              </svg>
              <span>Agora Debate</span>
            </button>
            <button 
              class="tab-pill survey-tab-pill"
              :class="{ active: activeTab === 'survey' }"
              @click="selectSurveyTab"
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M9 11l3 3L22 4"></path>
                <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"></path>
              </svg>
              <span>Quick Survey</span>
            </button>
          </div>
        </div>

        <!-- Chat Mode -->
        <div v-if="activeTab === 'chat'" class="chat-container">

          <!-- Report Agent Tools Card -->
          <div v-if="chatTarget === 'report_agent'" class="report-agent-tools-card">
            <div class="tools-card-header">
              <div class="tools-card-avatar">R</div>
              <div class="tools-card-info">
                <div class="tools-card-name">Report Agent - Chat</div>
                <div class="tools-card-subtitle">Quick chat version of Report Agent, can use 4 professional tools with full Popinion memory</div>
              </div>
              <button class="tools-card-toggle" @click="showToolsDetail = !showToolsDetail">
                <svg :class="{ 'is-expanded': showToolsDetail }" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
              </button>
            </div>
            <div v-if="showToolsDetail" class="tools-card-body">
              <div class="tools-grid">
                <div class="tool-item tool-purple">
                  <div class="tool-icon-wrapper">
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M9 18h6M10 22h4M12 2a7 7 0 0 0-4 12.5V17a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1v-2.5A7 7 0 0 0 12 2z"></path>
                    </svg>
                  </div>
                  <div class="tool-content">
                    <div class="tool-name">InsightForge Deep Analysis</div>
                    <div class="tool-desc">Aligns real-world seed data with simulation environment status, combining Global/Local Memory for deep cross-temporal analysis</div>
                  </div>
                </div>
                <div class="tool-item tool-blue">
                  <div class="tool-icon-wrapper">
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                      <circle cx="12" cy="12" r="10"></circle>
                      <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
                    </svg>
                  </div>
                  <div class="tool-content">
                    <div class="tool-name">PanoramaSearch Tracking</div>
                    <div class="tool-desc">Graph-based breadth traversal algorithm, reconstructs event propagation paths, captures complete info flow topology</div>
                  </div>
                </div>
                <div class="tool-item tool-orange">
                  <div class="tool-icon-wrapper">
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                      <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
                    </svg>
                  </div>
                  <div class="tool-content">
                    <div class="tool-name">QuickSearch Retrieval</div>
                    <div class="tool-desc">GraphRAG-based instant query interface, optimized indexing for quick extraction of specific node attributes and discrete facts</div>
                  </div>
                </div>
                <div class="tool-item tool-green">
                  <div class="tool-icon-wrapper">
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
                      <circle cx="9" cy="7" r="4"></circle>
                      <path d="M23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"></path>
                    </svg>
                  </div>
                  <div class="tool-content">
                    <div class="tool-name">InterviewSubAgent Virtual Interview</div>
                    <div class="tool-desc">Autonomous interviews that can conduct parallel multi-round conversations with agents, collecting unstructured perspective data and psychological states</div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Agent Profile Card -->
          <div v-if="chatTarget === 'agent' && selectedAgent" class="agent-profile-card">
            <div class="profile-card-header">
              <div class="profile-card-avatar">{{ (selectedAgent.username || 'A')[0] }}</div>
              <div class="profile-card-info">
                <div class="profile-card-name">{{ selectedAgent.username }}</div>
                <div class="profile-card-meta">
                  <span v-if="selectedAgent.name" class="profile-card-handle">@{{ selectedAgent.name }}</span>
                  <span class="profile-card-profession">{{ selectedAgent.profession || 'Unknown Profession' }}</span>
                </div>
              </div>
              <!-- Injection badge for single agent -->
              <button 
                v-if="getAgentInjectionCount(selectedAgentIndex) > 0 || globalInjectionCount > 0"
                class="agent-injection-badge"
                @click="openInjectionRemovalModal(selectedAgentIndex)"
                title="Click to view/remove injected knowledge"
              >
                🧠 {{ getAgentInjectionCount(selectedAgentIndex) + globalInjectionCount }}
              </button>
              <button class="profile-card-toggle" @click="showFullProfile = !showFullProfile">
                <svg :class="{ 'is-expanded': showFullProfile }" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
              </button>
            </div>
            <div v-if="showFullProfile && selectedAgent.bio" class="profile-card-body">
              <div class="profile-card-bio">
                <div class="profile-card-label">Bio</div>
                <p>{{ selectedAgent.bio }}</p>
              </div>
            </div>
          </div>

          <!-- Chat Messages -->
          <div class="chat-messages" ref="chatMessages">
            <div v-if="chatHistory.length === 0" class="chat-empty">
              <div class="empty-icon">
                <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
                </svg>
              </div>
              <p class="empty-text">
                {{ chatTarget === 'report_agent' ? 'Chat with Report Agent to understand report content in depth' : 'Chat with simulation agents to understand their perspective' }}
              </p>
            </div>
            <div 
              v-for="(msg, idx) in chatHistory" 
              :key="idx"
              class="chat-message"
              :class="msg.role"
            >
              <div class="message-avatar">
                <span v-if="msg.role === 'user'">U</span>
                <span v-else>{{ msg.role === 'assistant' && chatTarget === 'report_agent' ? 'R' : (selectedAgent?.username?.[0] || 'A') }}</span>
              </div>
              <div class="message-content">
                <div class="message-header">
                  <span class="sender-name">
                    {{ msg.role === 'user' ? 'You' : (chatTarget === 'report_agent' ? 'Report Agent' : (selectedAgent?.username || 'Agent')) }}
                  </span>
                  <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
                </div>
                <div class="message-text" v-html="renderMarkdown(msg.content)"></div>
              </div>
            </div>
            <div v-if="isSending" class="chat-message assistant">
              <div class="message-avatar">
                <span>{{ chatTarget === 'report_agent' ? 'R' : (selectedAgent?.username?.[0] || 'A') }}</span>
              </div>
              <div class="message-content">
                <div class="typing-indicator">
                  <span></span>
                  <span></span>
                  <span></span>
                </div>
              </div>
            </div>
          </div>

          <!-- Chat Input -->
          <div class="chat-input-area">
            <textarea 
              v-model="chatInput"
              class="chat-input"
              placeholder="Enter your question..."
              @keydown.enter.exact.prevent="sendMessage"
              :disabled="isSending || (!selectedAgent && chatTarget === 'agent')"
              rows="1"
              ref="chatInputRef"
            ></textarea>
            <button 
              class="send-btn"
              @click="sendMessage"
              :disabled="!chatInput.trim() || isSending || (!selectedAgent && chatTarget === 'agent')"
            >
              <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="22" y1="2" x2="11" y2="13"></line>
                <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
              </svg>
            </button>
          </div>
        </div>

        <!-- Panel Discussion Mode -->
        <div v-if="activeTab === 'panel'" class="panel-container">
          <!-- Participant Selection Header -->
          <div class="panel-header">
            <div class="participants-section">
              <div class="participants-label">
                <span class="label-text">Discussion Participants</span>
                <span class="participant-count">
                  {{ panelParticipants.size - mutedParticipants.size }} active / {{ panelParticipants.size }} total
                </span>
              </div>
              <div class="participants-chips">
                <!-- Global injection indicator (clickable) -->
                <button 
                  v-if="globalInjectionCount > 0" 
                  class="injection-indicator global clickable"
                  @click="openInjectionRemovalModal('global')"
                  title="Click to manage global injections"
                >
                  🧠 {{ globalInjectionCount }} global
                </button>
                <div 
                  v-for="idx in Array.from(panelParticipants).slice(0, 8)" 
                  :key="idx"
                  class="participant-chip"
                  :class="{ muted: mutedParticipants.has(idx) }"
                >
                  <span class="chip-avatar">{{ (profiles[idx]?.username || 'A')[0] }}</span>
                  <span class="chip-name">{{ profiles[idx]?.username || `Agent ${idx}` }}</span>
                  <!-- Injection badge (clickable) -->
                  <button 
                    v-if="getAgentInjectionCount(idx) > 0" 
                    class="chip-injection-badge clickable"
                    :title="`${getAgentInjectionCount(idx)} injected - click to manage`"
                    @click.stop="openInjectionRemovalModal(idx)"
                  >🧠{{ getAgentInjectionCount(idx) }}</button>
                  <button class="chip-mute" @click="toggleMute(idx)" :title="mutedParticipants.has(idx) ? 'Unmute' : 'Mute'">
                    {{ mutedParticipants.has(idx) ? '🔇' : '🔊' }}
                  </button>
                  <button class="chip-remove" @click="removeParticipant(idx)">×</button>
                </div>
                <div v-if="panelParticipants.size > 8" class="participant-chip more-chip">
                  +{{ panelParticipants.size - 8 }} more
                </div>
                <button class="add-participants-btn" @click="showParticipantModal = true">
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10"></circle>
                    <line x1="12" y1="8" x2="12" y2="16"></line>
                    <line x1="8" y1="12" x2="16" y2="12"></line>
                  </svg>
                  <span>{{ panelParticipants.size === 0 ? 'Select Participants' : 'Modify' }}</span>
                </button>
              </div>
            </div>
          </div>

          <!-- Conversation Thread -->
          <div class="panel-thread" ref="panelThreadRef">
            <div v-if="panelHistory.length === 0" class="empty-thread">
              <div class="empty-icon">💬</div>
              <p>Start a panel discussion by selecting participants and asking a question.</p>
            </div>
            
            <div v-for="(exchange, exchangeIdx) in panelHistory" :key="exchangeIdx" class="exchange-block">
              <!-- User Question -->
              <div class="exchange-question">
                <div class="question-bubble">
                  <span class="question-label">You asked:</span>
                  <p>{{ exchange.question }}</p>
                </div>
                <span class="exchange-time">{{ formatTime(exchange.timestamp) }}</span>
              </div>
              
              <!-- Panel Responses -->
              <div class="exchange-responses">
                <div 
                  v-for="(resp, respIdx) in exchange.responses" 
                  :key="respIdx"
                  class="response-card"
                >
                  <div class="resp-header">
                    <div class="resp-avatar">{{ (resp.agent_name || 'A')[0].toUpperCase() }}</div>
                    <div class="resp-info">
                      <span class="resp-name">{{ cleanAgentName(resp.agent_name) }}</span>
                      <span class="resp-role">{{ resp.profession || 'Participant' }}</span>
                    </div>
                    <button class="resp-reply-btn" @click="quoteResponse(resp, exchangeIdx)" title="Reply to this">
                      ↩ Reply
                    </button>
                  </div>
                  <div class="resp-content" v-html="renderMarkdown(resp.answer)"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- Input Area -->
          <div class="panel-input-area">
            <!-- Quote Indicator -->
            <div v-if="quotedResponse" class="quote-indicator">
              <span class="quote-text">Replying to {{ cleanAgentName(quotedResponse.agent_name) }}</span>
              <button class="quote-clear" @click="clearQuote">×</button>
            </div>
            <textarea 
              v-model="panelInput"
              class="panel-input"
              :placeholder="quotedResponse ? `Reply about ${cleanAgentName(quotedResponse.agent_name)}'s response...` : 'Ask the panel a question...'"
              rows="2"
              @keydown.enter.ctrl="sendPanelMessage"
              @keydown.enter.meta="sendPanelMessage"
            ></textarea>
            <button 
              class="panel-send-btn"
              :disabled="panelParticipants.size === 0 || !panelInput.trim() || isPanelSending"
              @click="sendPanelMessage"
            >
              <span v-if="isPanelSending" class="loading-spinner"></span>
              <span v-else>Ask Panel</span>
            </button>
          </div>
        </div>

        <!-- Participant Selection Modal -->
        <EntitySelectionModal
          :show="showParticipantModal"
          :entities="profilesAsEntities"
          :by-type="profilesByType"
          title="Select Panel Discussion Participants"
          item-label="participants"
          :show-estimate="false"
          :select-all-by-default="false"
          @close="showParticipantModal = false"
          @confirm="handleParticipantSelection"
        />

        <!-- Agora Debate Mode -->
        <div v-if="activeTab === 'agora'" class="agora-container">
          <AgoraPanel
            :simulation-id="simulationId"
            :profiles="profiles"
            @debate-started="handleDebateStarted"
            @debate-ended="handleDebateEnded"
            @add-log="addLog"
            @add-to-knowledge="handleAddToKnowledge"
          />
        </div>

        <!-- Quick Survey Mode -->
        <div v-if="activeTab === 'survey'" class="survey-container">
          <SurveyPanel
            :simulation-id="simulationId"
            :total-agents="profiles.length"
            @result="handleSurveyResult"
            @error="handleSurveyError"
          />
        </div>


        <!-- System Logs Panel -->
        <div class="system-logs-mini">
          <div class="logs-header">
            <span class="logs-title">Interaction Monitor</span>
            <button class="clear-logs-btn" @click="systemLogs.length = 0" v-if="systemLogs.length > 0">Clear</button>
          </div>
          <div class="logs-content" ref="logsContentRef">
            <div 
              v-for="(log, idx) in systemLogs" 
              :key="idx" 
              class="log-entry"
              :class="{ 'log-warning': log.msg.includes('⚠️'), 'log-error': log.msg.includes('❌') }"
            >
              <span class="log-entry-time">{{ log.time }}</span>
              <span class="log-entry-msg">{{ log.msg }}</span>
            </div>
            <div v-if="systemLogs.length === 0" class="logs-empty">No activity recorded yet</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { chatWithReport, getReport, getAgentLog } from '../api/report'
import { interviewAgents, getSimulationProfilesRealtime } from '../api/simulation'
import EntitySelectionModal from './EntitySelectionModal.vue'
import AgoraPanel from './AgoraPanel.vue'
import SurveyPanel from './SurveyPanel.vue'

const props = defineProps({
  reportId: String,
  simulationId: String,
  simulationTags: {
    type: Array,
    default: () => []
  },
  injectedKnowledge: {
    type: Object,
    default: () => ({})
  },
  systemLogs: {
    type: Array,
    default: () => []
  }
})

const emit = defineEmits(['add-log', 'update-status', 'add-to-knowledge', 'remove-injection', 'clear-injections'])

// State
const activeTab = ref('chat')
const chatTarget = ref('report_agent')
const showAgentDropdown = ref(false)
const selectedAgent = ref(null)
const selectedAgentIndex = ref(null)
const logsContentRef = ref(null)

// Watch for logs to scroll to bottom
watch(() => props.systemLogs.length, () => {
  nextTick(() => {
    if (logsContentRef.value) {
      logsContentRef.value.scrollTop = logsContentRef.value.scrollHeight
    }
  })
})
const showFullProfile = ref(true)
const showToolsDetail = ref(true)

// Text Selection State (for Knowledge Pad)
const selectionPopup = ref(null) // { x, y, text, source }
const currentSelectionSource = ref(null) // Track which content area is being selected from

// Tag Modal State
const showTagModal = ref(false)
const pendingHighlight = ref(null) // { text, source }
// Fixed categories for consistency across all simulations (custom tags can be added)
const predefinedTags = ['Economic', 'Political', 'Social', 'Risk', 'Opportunity', 'Consensus', 'Conflict', 'Insight']
const selectedTags = ref(new Set())
const customTagInput = ref('')

// Injection Removal Modal State
const showRemovalModal = ref(false)
const removalModalTarget = ref(null) // 'global', 'panel_all', or agent index
const removalModalItems = computed(() => {
  if (!removalModalTarget.value) return []
  
  if (removalModalTarget.value === 'global') {
    // Show both global and panel_all
    const globalItems = props.injectedKnowledge['global'] || []
    const panelItems = props.injectedKnowledge['panel_all'] || []
    return [...globalItems, ...panelItems]
  } else {
    // Agent-specific
    const key = typeof removalModalTarget.value === 'number' 
      ? `agent_${removalModalTarget.value}` 
      : removalModalTarget.value
    return props.injectedKnowledge[key] || []
  }
})

// Chat State
const chatInput = ref('')
const chatHistory = ref([])
const chatHistoryCache = ref({}) // Cached chat history: { 'report_agent': [], 'agent_0': [], 'agent_1': [], ... }
const isSending = ref(false)
const chatMessages = ref(null)
const chatInputRef = ref(null)

// Panel Discussion State (formerly Survey)
const panelParticipants = ref(new Set())  // Selected agent indices
const mutedParticipants = ref(new Set())  // Muted agents (still hear but don't respond)
const panelInput = ref('')  // Current question input
const panelHistory = ref([])  // Array of { question, responses: [{agent_id, agent_name, profession, answer}], timestamp }
const isPanelSending = ref(false)
const showParticipantModal = ref(false)
const quotedResponse = ref(null)  // { agent_name, answer, exchangeIdx } for click-to-quote

// Report Data
const reportOutline = ref(null)
const generatedSections = ref({})
const collapsedSections = ref(new Set())
const currentSectionIndex = ref(null)
const profiles = ref([])

// Helper Methods
const isSectionCompleted = (sectionIndex) => {
  return !!generatedSections.value[sectionIndex]
}

// Clean agent name: "hun_sen_319" → "Hun Sen"
const cleanAgentName = (name) => {
  if (!name) return 'Agent'
  return name
    .replace(/_\d+$/, '')           // Remove trailing numbers
    .replace(/_/g, ' ')              // Replace underscores with spaces
    .split(' ')
    .map(w => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
    .join(' ')
}

// Refs
const leftPanel = ref(null)
const rightPanel = ref(null)

// Methods
const addLog = (msg) => {
  emit('add-log', msg)
}

const toggleSectionCollapse = (idx) => {
  if (!generatedSections.value[idx + 1]) return
  const newSet = new Set(collapsedSections.value)
  if (newSet.has(idx)) {
    newSet.delete(idx)
  } else {
    newSet.add(idx)
  }
  collapsedSections.value = newSet
}

const selectChatTarget = (target) => {
  chatTarget.value = target
  if (target === 'report_agent') {
    showAgentDropdown.value = false
  }
}

// Save current chat history to cache
const saveChatHistory = () => {
  if (chatHistory.value.length === 0) return
  
  if (chatTarget.value === 'report_agent') {
    chatHistoryCache.value['report_agent'] = [...chatHistory.value]
  } else if (selectedAgentIndex.value !== null) {
    chatHistoryCache.value[`agent_${selectedAgentIndex.value}`] = [...chatHistory.value]
  }
}

const selectReportAgentChat = () => {
  // Save current chat history
  saveChatHistory()
  
  activeTab.value = 'chat'
  chatTarget.value = 'report_agent'
  selectedAgent.value = null
  selectedAgentIndex.value = null
  showAgentDropdown.value = false
  
  // Restore Report Agent chat history
  chatHistory.value = chatHistoryCache.value['report_agent'] || []
}

const selectSurveyTab = () => {
  activeTab.value = 'survey'
  selectedAgent.value = null
  selectedAgentIndex.value = null
  showAgentDropdown.value = false
}

const toggleAgentDropdown = () => {
  showAgentDropdown.value = !showAgentDropdown.value
  if (showAgentDropdown.value) {
    activeTab.value = 'chat'
    chatTarget.value = 'agent'
  }
}

const selectAgent = (agent, idx) => {
  // Save current chat history
  saveChatHistory()
  
  selectedAgent.value = agent
  selectedAgentIndex.value = idx
  chatTarget.value = 'agent'
  showAgentDropdown.value = false
  
  // Restore this Agent's chat history
  chatHistory.value = chatHistoryCache.value[`agent_${idx}`] || []
  addLog(`Select Chat Target: ${agent.username}`)
}

const formatTime = (timestamp) => {
  if (!timestamp) return ''
  try {
    return new Date(timestamp).toLocaleTimeString('en-US', { 
      hour12: false, 
      hour: '2-digit', 
      minute: '2-digit'
    })
  } catch {
    return ''
  }
}

const renderMarkdown = (content) => {
  if (!content) return ''
  
  let processedContent = content.replace(/^##\s+.+\n+/, '')
  let html = processedContent.replace(/```(\w*)\n([\s\S]*?)```/g, '<pre class="code-block"><code>$2</code></pre>')
  html = html.replace(/`([^`]+)`/g, '<code class="inline-code">$1</code>')
  html = html.replace(/^#### (.+)$/gm, '<h5 class="md-h5">$1</h5>')
  html = html.replace(/^### (.+)$/gm, '<h4 class="md-h4">$1</h4>')
  html = html.replace(/^## (.+)$/gm, '<h3 class="md-h3">$1</h3>')
  html = html.replace(/^# (.+)$/gm, '<h2 class="md-h2">$1</h2>')
  html = html.replace(/^> (.+)$/gm, '<blockquote class="md-quote">$1</blockquote>')
  
  // Process list - Support sub-list
  html = html.replace(/^(\s*)- (.+)$/gm, (match, indent, text) => {
    const level = Math.floor(indent.length / 2)
    return `<li class="md-li" data-level="${level}">${text}</li>`
  })
  html = html.replace(/^(\s*)(\d+)\. (.+)$/gm, (match, indent, num, text) => {
    const level = Math.floor(indent.length / 2)
    return `<li class="md-oli" data-level="${level}">${text}</li>`
  })
  
  // Wrap Unordered List
  html = html.replace(/(<li class="md-li"[^>]*>.*?<\/li>\s*)+/g, '<ul class="md-ul">$&</ul>')
  // Wrap Ordered List
  html = html.replace(/(<li class="md-oli"[^>]*>.*?<\/li>\s*)+/g, '<ol class="md-ol">$&</ol>')
  
  // Clean all whitespace between list items
  html = html.replace(/<\/li>\s+<li/g, '</li><li')
  // Clean whitespace after list start tag
  html = html.replace(/<ul class="md-ul">\s+/g, '<ul class="md-ul">')
  html = html.replace(/<ol class="md-ol">\s+/g, '<ol class="md-ol">')
  // Clean whitespace before list end tag
  html = html.replace(/\s+<\/ul>/g, '</ul>')
  html = html.replace(/\s+<\/ol>/g, '</ol>')
  
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>')
  html = html.replace(/_(.+?)_/g, '<em>$1</em>')
  html = html.replace(/^---$/gm, '<hr class="md-hr">')
  html = html.replace(/\n\n/g, '</p><p class="md-p">')
  html = html.replace(/\n/g, '<br>')
  html = '<p class="md-p">' + html + '</p>'
  html = html.replace(/<p class="md-p"><\/p>/g, '')
  html = html.replace(/<p class="md-p">(<h[2-5])/g, '$1')
  html = html.replace(/(<\/h[2-5]>)<\/p>/g, '$1')
  html = html.replace(/<p class="md-p">(<ul|<ol|<blockquote|<pre|<hr)/g, '$1')
  html = html.replace(/(<\/ul>|<\/ol>|<\/blockquote>|<\/pre>)<\/p>/g, '$1')
  // Clean <br> tags before and after list
  html = html.replace(/<br>\s*(<ul|<ol)/g, '$1')
  html = html.replace(/(<\/ul>|<\/ol>)\s*<br>/g, '$1')
  // Clean consecutive <br> tags
  html = html.replace(/(<br>\s*){2,}/g, '<br>')
  // Clean <br> before paragraph start tag immediately following list
  html = html.replace(/(<\/ol>|<\/ul>)<br>(<p|<div)/g, '$1$2')
  
  return html
}

// Chat Methods
const sendMessage = async () => {
  if (!chatInput.value.trim() || isSending.value) return
  
  const message = chatInput.value.trim()
  chatInput.value = ''
  
  // Add user message
  chatHistory.value.push({
    role: 'user',
    content: message,
    timestamp: new Date().toISOString()
  })
  
  scrollToBottom()
  isSending.value = true
  
  try {
    if (chatTarget.value === 'report_agent') {
      await sendToReportAgent(message)
    } else {
      await sendToAgent(message)
    }
  } catch (err) {
    addLog(`SendFailed: ${err.message}`)
    chatHistory.value.push({
      role: 'assistant',
      content: `Sorry, an error occurred: ${err.message}`,
      timestamp: new Date().toISOString()
    })
  } finally {
    isSending.value = false
    scrollToBottom()
    // Automatically save chat history to cache
    saveChatHistory()
  }
}

const sendToReportAgent = async (message) => {
  addLog(`Sending to Report Agent: ${message.substring(0, 50)}...`)
  
  // Build chat history for API
  const historyForApi = chatHistory.value
    .filter(msg => msg.role !== 'user' || msg.content !== message)
    .slice(-10) // Keep last 10 messages
    .map(msg => ({
      role: msg.role,
      content: msg.content
    }))
  
  const res = await chatWithReport({
    simulation_id: props.simulationId,
    message: message,
    chat_history: historyForApi
  })
  
  if (res.success && res.data) {
    chatHistory.value.push({
      role: 'assistant',
      content: res.data.response || res.data.answer || 'No Response',
      timestamp: new Date().toISOString()
    })
    addLog('Report Agent has replied')
  } else {
    throw new Error(res.error || 'Request Failed')
  }
}

const sendToAgent = async (message) => {
  if (!selectedAgent.value || selectedAgentIndex.value === null) {
    throw new Error('Please select a simulation agent first')
  }
  
  addLog(`Sending to ${selectedAgent.value.username}: ${message.substring(0, 50)}...`)
  
  // Build prompt with chat history
  let prompt = message
  if (chatHistory.value.length > 1) {
    const historyContext = chatHistory.value
      .filter(msg => msg.content !== message)
      .slice(-6)
      .map(msg => `${msg.role === 'user' ? 'Interviewer' : 'You'}: ${msg.content}`)
      .join('\n')
    prompt = `Here is our previous conversation:\n${historyContext}\n\nMy new question is: ${message}`
  }
  
  const res = await interviewAgents({
    simulation_id: props.simulationId,
    interviews: [{
      agent_id: selectedAgentIndex.value,
      prompt: prompt
    }]
  })
  
  if (res.success && res.data) {
    // Correct data path: res.data.result.results is an object dictionary
    // Format: {"twitter_0": {...}, "reddit_0": {...}} or single platform {"reddit_0": {...}}
    const resultData = res.data.result || res.data
    const resultsDict = resultData.results || resultData
    
    // Convert object dictionary to array, prioritize reddit platform reply
    let responseContent = null
    const agentId = selectedAgentIndex.value
    
    if (typeof resultsDict === 'object' && !Array.isArray(resultsDict)) {
      // Prioritize Reddit platform reply, then Twitter
      const redditKey = `reddit_${agentId}`
      const twitterKey = `twitter_${agentId}`
      const agentResult = resultsDict[redditKey] || resultsDict[twitterKey] || Object.values(resultsDict)[0]
      if (agentResult) {
        responseContent = agentResult.response || agentResult.answer
      }
    } else if (Array.isArray(resultsDict) && resultsDict.length > 0) {
      // CompatiblearrayFormat
      responseContent = resultsDict[0].response || resultsDict[0].answer
    }
    
    if (responseContent) {
      chatHistory.value.push({
        role: 'assistant',
        content: responseContent,
        timestamp: new Date().toISOString()
      })
      addLog(`${selectedAgent.value.username} has replied`)
    } else {
      throw new Error('No response data')
    }
  } else {
    throw new Error(res.error || 'Request Failed')
  }
}

const scrollToBottom = () => {
  nextTick(() => {
    if (chatMessages.value) {
      chatMessages.value.scrollTop = chatMessages.value.scrollHeight
    }
  })
}

// Panel Discussion Methods
const panelThreadRef = ref(null)

// Computed: Convert profiles to entity format for EntitySelectionModal
const profilesAsEntities = computed(() => {
  return profiles.value.map((p, idx) => ({
    uuid: String(idx),  // Use uuid to match EntitySelectionModal expectations
    name: p.username || `Agent ${idx}`,
    type: p.profession || 'Participant',
    relationship_count: 0
  }))
})

// Computed: Group profiles by profession for EntitySelectionModal
const profilesByType = computed(() => {
  const grouped = {}
  profiles.value.forEach((p, idx) => {
    const type = p.profession || 'Other'
    if (!grouped[type]) {
      grouped[type] = { count: 0, entities: [] }
    }
    grouped[type].count++
    grouped[type].entities.push({
      uuid: String(idx),  // Use uuid to match EntitySelectionModal expectations
      name: p.username || `Agent ${idx}`,
      type: type
    })
  })
  return grouped
})

// Computed for injection badge counts
const globalInjectionCount = computed(() => {
  const globalCount = props.injectedKnowledge['global']?.length || 0
  const panelCount = props.injectedKnowledge['panel_all']?.length || 0
  return globalCount + panelCount
})

const getAgentInjectionCount = (idx) => {
  const agentKey = `agent_${idx}`
  return props.injectedKnowledge[agentKey]?.length || 0
}

// Injection removal modal methods
const openInjectionRemovalModal = (target) => {
  removalModalTarget.value = target
  showRemovalModal.value = true
}

const closeRemovalModal = () => {
  showRemovalModal.value = false
  removalModalTarget.value = null
}

const removeInjection = (target, index) => {
  // Emit to parent to actually remove from state
  emit('remove-injection', { target, index })
  addLog(`Removed injection item at index ${index}`)
}

const clearAllInjections = (target) => {
  emit('clear-injections', { target })
  addLog(`Cleared all injections for ${target}`)
  closeRemovalModal()
}

const selectPanelTab = () => {
  activeTab.value = 'panel'
  selectedAgent.value = null
  selectedAgentIndex.value = null
  showAgentDropdown.value = false
}

const selectAgoraTab = () => {
  activeTab.value = 'agora'
  selectedAgent.value = null
  selectedAgentIndex.value = null
  showAgentDropdown.value = false
}

// Agora debate state
const agoraDebateActive = ref(false)

const handleDebateStarted = (debateId) => {
  agoraDebateActive.value = true
  addLog(`Agora debate started: ${debateId}`)
}

const handleDebateEnded = (debateId) => {
  agoraDebateActive.value = false
  addLog(`Agora debate ended: ${debateId}`)
}

// Survey handlers
const handleSurveyResult = (result) => {
  addLog(`Survey completed: ${result.total_respondents} respondents`)
}

const handleSurveyError = (err) => {
  addLog(`⚠️ Survey error: ${err.message || 'Unknown error'}`)
}

const removeParticipant = (idx) => {
  const newSet = new Set(panelParticipants.value)
  newSet.delete(idx)
  panelParticipants.value = newSet
  // Also remove from muted if present
  if (mutedParticipants.value.has(idx)) {
    const mutedSet = new Set(mutedParticipants.value)
    mutedSet.delete(idx)
    mutedParticipants.value = mutedSet
  }
}

const toggleMute = (idx) => {
  const newSet = new Set(mutedParticipants.value)
  if (newSet.has(idx)) {
    newSet.delete(idx)  // Unmute
    addLog(`Unmuted ${profiles.value[idx]?.username || 'Agent'}`)
  } else {
    newSet.add(idx)     // Mute
    addLog(`Muted ${profiles.value[idx]?.username || 'Agent'}`)
  }
  mutedParticipants.value = newSet
}

const handleParticipantSelection = (selectedIds) => {
  // selectedIds are string IDs from the modal, convert to numbers
  const newSet = new Set(selectedIds.map(id => parseInt(id, 10)))
  panelParticipants.value = newSet
  showParticipantModal.value = false
  addLog(`Selected ${newSet.size} participants for panel discussion`)
}

// Quote a response for explicit reference
const quoteResponse = (response, exchangeIdx) => {
  quotedResponse.value = {
    agent_name: response.agent_name,
    answer: response.answer,
    exchangeIdx: exchangeIdx
  }
  addLog(`Quoting ${cleanAgentName(response.agent_name)}'s response`)
}

const clearQuote = () => {
  quotedResponse.value = null
}

// --- Text Selection for Knowledge Pad ---
let selectionTimeout = null

const checkSelection = () => {
  const selection = window.getSelection()
  const text = selection.toString().trim()
  
  if (text.length > 15) {
    const range = selection.getRangeAt(0)
    const rect = range.getBoundingClientRect()
    
    // Determine source from selection container
    const container = range.commonAncestorContainer.parentElement
    let source = { agent: 'Unknown', context: 'unknown' }
    
    // Check if selection is in panel discussion
    const panelCard = container?.closest('.response-card')
    if (panelCard) {
      const nameEl = panelCard.querySelector('.resp-name')
      source = { agent: nameEl?.textContent || 'Panel Agent', context: 'panel' }
    }
    
    // Check if selection is in chat
    const chatMsg = container?.closest('.chat-message')
    if (chatMsg) {
      const isUser = chatMsg.classList.contains('user')
      if (!isUser) {
        source = { agent: chatTarget.value === 'report_agent' ? 'Report Agent' : (selectedAgent.value?.username || 'Agent'), context: chatTarget.value === 'report_agent' ? 'report_chat' : 'agent_chat' }
      }
    }
    
    if (source.context !== 'unknown') {
      selectionPopup.value = {
        x: rect.left + rect.width / 2,
        y: rect.top - 10,
        text: text,
        source: source
      }
    }
  } else {
    selectionPopup.value = null
  }
}

// Debounced selection handler - fires 300ms after selection stops changing
const handleTextSelection = () => {
  clearTimeout(selectionTimeout)
  selectionTimeout = setTimeout(checkSelection, 300)
}

const openTagModal = () => {
  if (!selectionPopup.value) return
  pendingHighlight.value = {
    text: selectionPopup.value.text,
    source: selectionPopup.value.source
  }
  showTagModal.value = true
  selectedTags.value = new Set()
  customTagInput.value = ''
}

const closeTagModal = () => {
  showTagModal.value = false
  pendingHighlight.value = null
  selectionPopup.value = null
  window.getSelection().removeAllRanges()
}

const toggleTag = (tag) => {
  const newSet = new Set(selectedTags.value)
  if (newSet.has(tag)) {
    newSet.delete(tag)
  } else {
    newSet.add(tag)
  }
  selectedTags.value = newSet
}

const addCustomTag = () => {
  const tag = customTagInput.value.trim()
  if (tag && tag.length > 0) {
    const newSet = new Set(selectedTags.value)
    newSet.add(tag)
    selectedTags.value = newSet
    customTagInput.value = ''
  }
}

const confirmAddToKnowledge = () => {
  if (!pendingHighlight.value) return
  
  emit('add-to-knowledge', {
    content: pendingHighlight.value.text,
    source: pendingHighlight.value.source,
    tags: Array.from(selectedTags.value)
  })
  
  addLog(`Added to Knowledge: "${pendingHighlight.value.text.substring(0, 40)}..." with ${selectedTags.value.size} tag(s)`)
  closeTagModal()
}

// Handle add-to-knowledge emit from child components (e.g., AgoraPanel)
const handleAddToKnowledge = (data) => {
  emit('add-to-knowledge', data)
  addLog(`Added to Knowledge: "${data.content.substring(0, 40)}..." from ${data.source}`)
}

const clearSelection = () => {
  // Only clear if there's no active text selection and tag modal is not open
  if (showTagModal.value) return
  const selection = window.getSelection()
  const text = selection.toString().trim()
  if (text.length < 5) {
    selectionPopup.value = null
  }
}

const sendPanelMessage = async () => {
  // Filter out muted participants
  const activeParticipants = Array.from(panelParticipants.value)
    .filter(idx => !mutedParticipants.value.has(idx))
  
  if (activeParticipants.length === 0 || !panelInput.value.trim()) {
    if (panelParticipants.value.size > 0 && activeParticipants.length === 0) {
      addLog('All participants are muted. Unmute at least one to send a question.')
    }
    return
  }
  
  isPanelSending.value = true
  const question = panelInput.value.trim()
  addLog(`Sending question to ${activeParticipants.length} active panel members...`)
  
  try {
    let promptWithContext = question
    
    // Check if this is a focused reply (quote active) vs general question
    if (quotedResponse.value) {
      // FOCUSED REPLY: Only include the quoted response, no general history
      const quoteName = cleanAgentName(quotedResponse.value.agent_name)
      const quoteContent = quotedResponse.value.answer.substring(0, 500)
      promptWithContext = `You are being asked to respond specifically to what ${quoteName} said.\n\n` +
        `${quoteName}'s statement:\n"${quoteContent}"\n\n` +
        `Question about this statement: ${question}`
      quotedResponse.value = null  // Clear after use
      addLog(`Focused reply about ${quoteName}'s response`)
    } else if (panelHistory.value.length > 0) {
      // GENERAL QUESTION: Include recent history context
      const historyContext = panelHistory.value.slice(-3).map(exchange => 
        `Previous question: "${exchange.question}"\n` + 
        exchange.responses.slice(0, 3).map(r => 
          `${cleanAgentName(r.agent_name)}: ${r.answer.substring(0, 200)}...`
        ).join('\n')
      ).join('\n---\n')
      promptWithContext = `Context from our previous discussion:\n${historyContext}\n\nNew question: ${question}`
    }
    
    const interviews = activeParticipants.map(idx => {
      // Collect all injected knowledge for this agent
      const agentKnowledge = []
      
      // 1. Global knowledge (applies to all)
      if (props.injectedKnowledge['global']?.length) {
        agentKnowledge.push(...props.injectedKnowledge['global'])
      }
      
      // 2. Panel-wide knowledge
      if (props.injectedKnowledge['panel_all']?.length) {
        agentKnowledge.push(...props.injectedKnowledge['panel_all'])
      }
      
      // 3. Agent-specific knowledge
      const agentKey = `agent_${idx}`
      if (props.injectedKnowledge[agentKey]?.length) {
        agentKnowledge.push(...props.injectedKnowledge[agentKey])
      }
      
      // Build final prompt with injected knowledge prepended
      let finalPrompt = promptWithContext
      if (agentKnowledge.length > 0) {
        const knowledgeContext = agentKnowledge.map((k, i) => `${i + 1}. ${k}`).join('\n')
        finalPrompt = `[Important context to consider in your response]:\n${knowledgeContext}\n\n---\n\n${promptWithContext}`
        addLog(`Agent #${idx} prompt includes ${agentKnowledge.length} injected knowledge item(s)`)
      }
      
      return {
        agent_id: idx,
        prompt: finalPrompt
      }
    })
    
    const res = await interviewAgents({
      simulation_id: props.simulationId,
      interviews: interviews
    })
    
    if (res.success && res.data) {
      const resultData = res.data.result || res.data
      const resultsDict = resultData.results || resultData
      
      // Build responses array
      const responses = []
      
      for (const interview of interviews) {
        const agentIdx = interview.agent_id
        const agent = profiles.value[agentIdx]
        
        let responseContent = 'No Response'
        
        if (typeof resultsDict === 'object' && !Array.isArray(resultsDict)) {
          const redditKey = `reddit_${agentIdx}`
          const twitterKey = `twitter_${agentIdx}`
          const agentResult = resultsDict[redditKey] || resultsDict[twitterKey]
          if (agentResult) {
            responseContent = agentResult.response || agentResult.answer || 'No Response'
          }
        } else if (Array.isArray(resultsDict)) {
          const matchedResult = resultsDict.find(r => r.agent_id === agentIdx)
          if (matchedResult) {
            responseContent = matchedResult.response || matchedResult.answer || 'No Response'
          }
        }
        
        responses.push({
          agent_id: agentIdx,
          agent_name: agent?.username || `Agent ${agentIdx}`,
          profession: agent?.profession,
          answer: responseContent
        })
      }
      
      // Append to history instead of replacing
      panelHistory.value.push({
        question: question,
        responses: responses,
        timestamp: new Date().toISOString()
      })
      
      panelInput.value = ''  // Clear input for next question
      addLog(`Received ${responses.length} panel responses`)
      
      // Scroll to bottom
      nextTick(() => {
        if (panelThreadRef.value) {
          panelThreadRef.value.scrollTop = panelThreadRef.value.scrollHeight
        }
      })
    } else {
      throw new Error(res.error || 'Request Failed')
    }
  } catch (err) {
    addLog(`Panel discussion failed: ${err.message}`)
  } finally {
    isPanelSending.value = false
  }
}

// Load Report Data
const loadReportData = async () => {
  if (!props.reportId) return
  
  try {
    addLog(`Loading report data: ${props.reportId}`)
    
    // Get report info
    const reportRes = await getReport(props.reportId)
    if (reportRes.success && reportRes.data) {
      // Load agent logs to get report outline and sections
      await loadAgentLogs()
    }
  } catch (err) {
    addLog(`Failed to load report: ${err.message}`)
  }
}

const loadAgentLogs = async () => {
  if (!props.reportId) return
  
  try {
    const res = await getAgentLog(props.reportId, 0)
    if (res.success && res.data) {
      const logs = res.data.logs || []
      
      logs.forEach(log => {
        if (log.action === 'planning_complete' && log.details?.outline) {
          reportOutline.value = log.details.outline
        }
        
        if (log.action === 'section_complete' && log.section_index < 100 && log.details?.content) {
          generatedSections.value[log.section_index] = log.details.content
        }
      })
      
      addLog('Report data loading completed')
    }
  } catch (err) {
    addLog(`Failed to load report logs: ${err.message}`)
  }
}

const loadProfiles = async () => {
  if (!props.simulationId) return
  
  try {
    const res = await getSimulationProfilesRealtime(props.simulationId, 'reddit')
    if (res.success && res.data) {
      profiles.value = res.data.profiles || []
      addLog(`Loading ${profiles.value.length} simulation agents`)
    }
  } catch (err) {
    addLog(`Failed to load simulation agents: ${err.message}`)
  }
}

// Click outside to close dropdown
const handleClickOutside = (e) => {
  const dropdown = document.querySelector('.agent-dropdown')
  if (dropdown && !dropdown.contains(e.target)) {
    showAgentDropdown.value = false
  }
}

// Lifecycle
onMounted(() => {
  addLog('Step5 Deep Interaction Initializing')
  loadReportData()
  loadProfiles()
  document.addEventListener('click', handleClickOutside)
  document.addEventListener('selectionchange', handleTextSelection)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
  document.removeEventListener('selectionchange', handleTextSelection)
  clearTimeout(selectionTimeout)
})

watch(() => props.reportId, (newId) => {
  if (newId) {
    loadReportData()
  }
}, { immediate: true })

watch(() => props.simulationId, (newId) => {
  if (newId) {
    loadProfiles()
  }
}, { immediate: true })
</script>

<style scoped>
.interaction-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #F8F9FA;
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  overflow: hidden;
}

/* Utility Classes */
.mono {
  font-family: 'JetBrains Mono', 'SF Mono', 'Monaco', 'Consolas', monospace;
}

/* Main Split Layout */
.main-split-layout {
  flex: 1;
  display: flex;
  overflow: hidden;
}

/* Left Panel - Report Style (Fully consistent with Step4Report.vue) */
.left-panel.report-style {
  width: 45%;
  min-width: 450px;
  background: #FFFFFF;
  border-right: 1px solid #E5E7EB;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  padding: 30px 50px 60px 50px;
  transition: all 0.3s ease;
}

/* Agora Mode - Hide left panel, expand right panel */
.main-split-layout.agora-mode .left-panel {
  width: 0;
  min-width: 0;
  padding: 0;
  opacity: 0;
  overflow: hidden;
  border: none;
}

.main-split-layout.agora-mode .right-panel {
  width: 100%;
}

.left-panel::-webkit-scrollbar {
  width: 6px;
}

.left-panel::-webkit-scrollbar-track {
  background: transparent;
}

.left-panel::-webkit-scrollbar-thumb {
  background: transparent;
  border-radius: 3px;
  transition: background 0.3s ease;
}

.left-panel:hover::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
}

.left-panel::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.25);
}

/* Report Header */
.report-content-wrapper {
  max-width: 800px;
  margin: 0 auto;
  width: 100%;
}

.report-header-block {
  margin-bottom: 30px;
}

.report-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}

.report-tag {
  background: #000000;
  color: #FFFFFF;
  font-size: 11px;
  font-weight: 700;
  padding: 4px 8px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.report-id {
  font-size: 11px;
  color: #9CA3AF;
  font-weight: 500;
  letter-spacing: 0.02em;
}

.main-title {
  font-family: 'Times New Roman', Times, serif;
  font-size: 36px;
  font-weight: 700;
  color: #111827;
  line-height: 1.2;
  margin: 0 0 16px 0;
  letter-spacing: -0.02em;
}

.sub-title {
  font-family: 'Times New Roman', Times, serif;
  font-size: 16px;
  color: #6B7280;
  font-style: italic;
  line-height: 1.6;
  margin: 0 0 30px 0;
  font-weight: 400;
}

.header-divider {
  height: 1px;
  background: #E5E7EB;
  width: 100%;
}

/* Sections List */
.sections-list {
  display: flex;
  flex-direction: column;
  gap: 32px;
}

.report-section-item {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-header-row {
  display: flex;
  align-items: baseline;
  gap: 12px;
  transition: background-color 0.2s ease;
  padding: 8px 12px;
  margin: -8px -12px;
  border-radius: 8px;
}

.section-header-row.clickable {
  cursor: pointer;
}

.section-header-row.clickable:hover {
  background-color: #F9FAFB;
}

.collapse-icon {
  margin-left: auto;
  color: #9CA3AF;
  transition: transform 0.3s ease;
  flex-shrink: 0;
  align-self: center;
}

.collapse-icon.is-collapsed {
  transform: rotate(-90deg);
}

.section-number {
  font-family: 'JetBrains Mono', monospace;
  font-size: 16px;
  color: #E5E7EB;
  font-weight: 500;
  transition: color 0.3s ease;
}

.section-title {
  font-family: 'Times New Roman', Times, serif;
  font-size: 24px;
  font-weight: 600;
  color: #111827;
  margin: 0;
  transition: color 0.3s ease;
}

/* States */
.report-section-item.is-pending .section-number {
  color: #E5E7EB;
}
.report-section-item.is-pending .section-title {
  color: #D1D5DB;
}

.report-section-item.is-active .section-number,
.report-section-item.is-completed .section-number {
  color: #9CA3AF;
}

.report-section-item.is-active .section-title,
.report-section-item.is-completed .section-title {
  color: #111827;
}

.section-body {
  padding-left: 28px;
  overflow: hidden;
}

/* Generated Content */
.generated-content {
  font-family: 'Inter', -apple-system, sans-serif;
  font-size: 14px;
  line-height: 1.8;
  color: #374151;
}

.generated-content :deep(p) {
  margin-bottom: 1em;
}

.generated-content :deep(.md-h2),
.generated-content :deep(.md-h3),
.generated-content :deep(.md-h4) {
  font-family: 'Times New Roman', Times, serif;
  color: #111827;
  margin-top: 1.5em;
  margin-bottom: 0.8em;
  font-weight: 700;
}

.generated-content :deep(.md-h2) { font-size: 20px; border-bottom: 1px solid #F3F4F6; padding-bottom: 8px; }
.generated-content :deep(.md-h3) { font-size: 18px; }
.generated-content :deep(.md-h4) { font-size: 16px; }

.generated-content :deep(.md-ul),
.generated-content :deep(.md-ol) {
  padding-left: 20px;
  margin-bottom: 1em;
}

.generated-content :deep(.md-li) {
  margin-bottom: 0.5em;
}

.generated-content :deep(.md-quote) {
  border-left: 3px solid #E5E7EB;
  padding-left: 16px;
  margin: 1.5em 0;
  color: #6B7280;
  font-style: italic;
  font-family: 'Times New Roman', Times, serif;
}

.generated-content :deep(.code-block) {
  background: #F9FAFB;
  padding: 12px;
  border-radius: 6px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  overflow-x: auto;
  margin: 1em 0;
  border: 1px solid #E5E7EB;
}

.generated-content :deep(strong) {
  font-weight: 600;
  color: #111827;
}

/* Loading State */
.loading-state {
  display: flex;
  align-items: center;
  gap: 10px;
  color: #6B7280;
  font-size: 14px;
  margin-top: 4px;
}

.loading-icon {
  width: 18px;
  height: 18px;
  animation: spin 1s linear infinite;
  display: flex;
  align-items: center;
  justify-content: center;
}

.loading-text {
  font-family: 'Times New Roman', Times, serif;
  font-size: 15px;
  color: #4B5563;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Content Styles Override */
.generated-content :deep(.md-h2) {
  font-family: 'Times New Roman', Times, serif;
  font-size: 18px;
  margin-top: 0;
}

/* Waiting Placeholder */
.waiting-placeholder {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 20px;
  padding: 40px;
  color: #9CA3AF;
}

.waiting-animation {
  position: relative;
  width: 48px;
  height: 48px;
}

.waiting-ring {
  position: absolute;
  width: 100%;
  height: 100%;
  border: 2px solid #E5E7EB;
  border-radius: 50%;
  animation: ripple 2s cubic-bezier(0.4, 0, 0.2, 1) infinite;
}

.waiting-ring:nth-child(2) {
  animation-delay: 0.4s;
}

.waiting-ring:nth-child(3) {
  animation-delay: 0.8s;
}

@keyframes ripple {
  0% { transform: scale(0.5); opacity: 1; }
  100% { transform: scale(2); opacity: 0; }
}

.waiting-text {
  font-size: 14px;
}

/* Right Panel - Interaction */
.right-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: #FFFFFF;
  overflow: hidden;
}

/* Action Bar - Professional Design */
.action-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-bottom: 1px solid #E5E7EB;
  background: linear-gradient(180deg, #FFFFFF 0%, #FAFBFC 100%);
  gap: 16px;
}

.action-bar-header {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 160px;
}

.action-bar-icon {
  color: #1F2937;
  flex-shrink: 0;
}

.action-bar-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.action-bar-title {
  font-size: 13px;
  font-weight: 600;
  color: #1F2937;
  letter-spacing: -0.01em;
}

.action-bar-subtitle {
  font-size: 11px;
  color: #9CA3AF;
}

.action-bar-subtitle.mono {
  font-family: 'JetBrains Mono', 'SF Mono', monospace;
}

.action-bar-tabs {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  justify-content: flex-end;
}

.tab-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  font-size: 12px;
  font-weight: 500;
  color: #374151;
  background: #F3F4F6;
  border: 2px solid #E5E7EB;
  border-radius: 20px;
  cursor: pointer;
  transition: all 0.2s ease;
  white-space: nowrap;
}

.tab-pill:hover {
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.tab-pill.active {
  background: #1F2937;
  color: #FFFFFF;
  border-color: #1F2937;
  box-shadow: 0 2px 8px rgba(31, 41, 55, 0.2);
}

.tab-pill svg {
  flex-shrink: 0;
  opacity: 0.7;
}

.tab-pill.active svg {
  opacity: 1;
}

.tab-divider {
  width: 1px;
  height: 24px;
  background: #E5E7EB;
  margin: 0 6px;
}

.agent-pill {
  width: 200px;
  justify-content: space-between;
  background: #EFF6FF;
  color: #1D4ED8;
  border-color: #BFDBFE;
}

.agent-pill:hover {
  background: #DBEAFE;
  border-color: #93C5FD;
}

.agent-pill.active {
  background: #1D4ED8;
  color: #FFFFFF;
  border-color: #1D4ED8;
  box-shadow: 0 2px 8px rgba(29, 78, 216, 0.25);
}

.agent-pill span {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}

/* Report Agent Chat - Orange accent */
.tab-pill:not(.agent-pill):not(.survey-pill):not(.agora-pill) {
  background: #FFF7ED;
  color: #C2410C;
  border-color: #FED7AA;
}

.tab-pill:not(.agent-pill):not(.survey-pill):not(.agora-pill):hover {
  background: #FFEDD5;
  border-color: #FDBA74;
}

.tab-pill:not(.agent-pill):not(.survey-pill):not(.agora-pill).active {
  background: #EA580C;
  color: #FFFFFF;
  border-color: #EA580C;
  box-shadow: 0 2px 8px rgba(234, 88, 12, 0.25);
}

/* Panel Discussion - Green accent */
.survey-pill {
  background: #ECFDF5;
  color: #047857;
  border-color: #A7F3D0;
}

.survey-pill:hover {
  background: #D1FAE5;
  border-color: #6EE7B7;
}

.survey-pill.active {
  background: #047857;
  color: #FFFFFF;
  border-color: #047857;
  box-shadow: 0 2px 8px rgba(4, 120, 87, 0.25);
}

/* Agora Debate - Purple accent */
.agora-pill {
  background: #F5F3FF;
  color: #7C3AED;
  border-color: #DDD6FE;
}

.agora-pill:hover {
  background: #EDE9FE;
  border-color: #C4B5FD;
}

.agora-pill.active {
  background: #7C3AED;
  color: #FFFFFF;
  border-color: #7C3AED;
  box-shadow: 0 2px 8px rgba(124, 58, 237, 0.25);
}

/* Interaction Header */
.interaction-header {
  padding: 16px 24px;
  border-bottom: 1px solid #E5E7EB;
  background: #FAFAFA;
}

.tab-switcher {
  display: flex;
  gap: 8px;
}

.tab-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  font-size: 13px;
  font-weight: 600;
  color: #6B7280;
  background: transparent;
  border: 1px solid #E5E7EB;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.tab-btn:hover {
  background: #F9FAFB;
  border-color: #D1D5DB;
}

.tab-btn.active {
  background: #1F2937;
  color: #FFFFFF;
  border-color: #1F2937;
}

.tab-btn svg {
  flex-shrink: 0;
}

/* Chat Container */
.chat-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Report Agent Tools Card */
.report-agent-tools-card {
  border-bottom: 1px solid #E5E7EB;
  background: linear-gradient(135deg, #F8FAFC 0%, #F1F5F9 100%);
}

.tools-card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 20px;
}

.tools-card-avatar {
  width: 44px;
  height: 44px;
  min-width: 44px;
  min-height: 44px;
  background: linear-gradient(135deg, #1F2937 0%, #374151 100%);
  color: #FFFFFF;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  font-weight: 600;
  flex-shrink: 0;
  box-shadow: 0 2px 8px rgba(31, 41, 55, 0.2);
}

.tools-card-info {
  flex: 1;
  min-width: 0;
}

.tools-card-name {
  font-size: 15px;
  font-weight: 600;
  color: #1F2937;
  margin-bottom: 2px;
}

.tools-card-subtitle {
  font-size: 12px;
  color: #6B7280;
}

.tools-card-toggle {
  width: 28px;
  height: 28px;
  background: #FFFFFF;
  border: 1px solid #E5E7EB;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #6B7280;
  transition: all 0.2s ease;
  flex-shrink: 0;
}

.tools-card-toggle:hover {
  background: #F9FAFB;
  border-color: #D1D5DB;
}

.tools-card-toggle svg {
  transition: transform 0.3s ease;
}

.tools-card-toggle svg.is-expanded {
  transform: rotate(180deg);
}

.tools-card-body {
  padding: 0 20px 16px 20px;
}

.tools-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}

.tool-item {
  display: flex;
  gap: 10px;
  padding: 12px;
  background: #FFFFFF;
  border-radius: 10px;
  border: 1px solid #E5E7EB;
  transition: all 0.2s ease;
}

.tool-item:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
}

.tool-icon-wrapper {
  width: 32px;
  height: 32px;
  min-width: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.tool-purple .tool-icon-wrapper {
  background: rgba(255, 87, 34, 0.1);
  color: #8B5CF6;
}

.tool-blue .tool-icon-wrapper {
  background: rgba(59, 130, 246, 0.1);
  color: #3B82F6;
}

.tool-orange .tool-icon-wrapper {
  background: rgba(249, 115, 22, 0.1);
  color: #F97316;
}

.tool-green .tool-icon-wrapper {
  background: rgba(34, 197, 94, 0.1);
  color: #22C55E;
}

.tool-content {
  flex: 1;
  min-width: 0;
}

.tool-name {
  font-size: 12px;
  font-weight: 600;
  color: #1F2937;
  margin-bottom: 4px;
}

.tool-desc {
  font-size: 11px;
  color: #6B7280;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* Agent Profile Card */
.agent-profile-card {
  border-bottom: 1px solid #E5E7EB;
  background: linear-gradient(135deg, #F8FAFC 0%, #F1F5F9 100%);
}

.profile-card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 20px;
}

.profile-card-avatar {
  width: 44px;
  height: 44px;
  min-width: 44px;
  min-height: 44px;
  background: linear-gradient(135deg, #1F2937 0%, #374151 100%);
  color: #FFFFFF;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  font-weight: 600;
  flex-shrink: 0;
  box-shadow: 0 2px 8px rgba(31, 41, 55, 0.2);
}

.profile-card-info {
  flex: 1;
  min-width: 0;
}

.profile-card-name {
  font-size: 15px;
  font-weight: 600;
  color: #1F2937;
  margin-bottom: 2px;
}

.profile-card-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #6B7280;
}

.profile-card-handle {
  color: #9CA3AF;
}

.profile-card-profession {
  padding: 2px 8px;
  background: #E5E7EB;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 500;
}

.profile-card-toggle {
  width: 28px;
  height: 28px;
  background: #FFFFFF;
  border: 1px solid #E5E7EB;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #6B7280;
  transition: all 0.2s ease;
  flex-shrink: 0;
}

.profile-card-toggle:hover {
  background: #F9FAFB;
  border-color: #D1D5DB;
}

.profile-card-toggle svg {
  transition: transform 0.3s ease;
}

.profile-card-toggle svg.is-expanded {
  transform: rotate(180deg);
}

.profile-card-body {
  padding: 0 20px 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.profile-card-label {
  font-size: 11px;
  font-weight: 600;
  color: #9CA3AF;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 6px;
}

.profile-card-bio {
  background: #FFFFFF;
  padding: 12px 14px;
  border-radius: 8px;
  border: 1px solid #E5E7EB;
}

.profile-card-bio p {
  margin: 0;
  font-size: 13px;
  line-height: 1.6;
  color: #4B5563;
}

/* Target Selector */
.target-selector {
  padding: 16px 24px;
  border-bottom: 1px solid #E5E7EB;
}

.selector-label {
  font-size: 11px;
  font-weight: 600;
  color: #9CA3AF;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 10px;
}

.selector-options {
  display: flex;
  gap: 12px;
}

.target-option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  font-size: 13px;
  font-weight: 500;
  color: #374151;
  background: #F9FAFB;
  border: 1px solid #E5E7EB;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.target-option:hover {
  border-color: #D1D5DB;
}

.target-option.active {
  background: #1F2937;
  color: #FFFFFF;
  border-color: #1F2937;
}

/* Agent Dropdown */
.agent-dropdown {
  position: relative;
}

.dropdown-arrow {
  margin-left: 4px;
  transition: transform 0.2s ease;
  opacity: 0.6;
}

.dropdown-arrow.open {
  transform: rotate(180deg);
}

.dropdown-menu {
  position: absolute;
  top: calc(100% + 6px);
  left: 50%;
  transform: translateX(-50%);
  min-width: 240px;
  background: #FFFFFF;
  border: 1px solid #E5E7EB;
  border-radius: 12px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.12), 0 4px 12px rgba(0, 0, 0, 0.06);
  max-height: 320px;
  overflow-y: auto;
  z-index: 100;
}

.dropdown-header {
  padding: 12px 16px 8px;
  font-size: 11px;
  font-weight: 600;
  color: #9CA3AF;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-bottom: 1px solid #F3F4F6;
}

.dropdown-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  cursor: pointer;
  transition: all 0.15s ease;
  border-left: 3px solid transparent;
}

.dropdown-item:hover {
  background: #F9FAFB;
  border-left-color: #1F2937;
}

.dropdown-item:first-of-type {
  margin-top: 4px;
}

.dropdown-item:last-child {
  margin-bottom: 4px;
}

.agent-avatar {
  width: 32px;
  height: 32px;
  min-width: 32px;
  min-height: 32px;
  background: linear-gradient(135deg, #1F2937 0%, #374151 100%);
  color: #FFFFFF;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
  flex-shrink: 0;
  box-shadow: 0 2px 4px rgba(31, 41, 55, 0.1);
}

.agent-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}

.agent-name {
  font-size: 13px;
  font-weight: 600;
  color: #1F2937;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.agent-role {
  font-size: 11px;
  color: #9CA3AF;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Chat Messages */
.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.chat-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  color: #9CA3AF;
}

.empty-icon {
  opacity: 0.3;
}

.empty-text {
  font-size: 14px;
  text-align: center;
  max-width: 280px;
  line-height: 1.6;
}

.chat-message {
  display: flex;
  gap: 12px;
}

.chat-message.user {
  flex-direction: row-reverse;
}

.message-avatar {
  width: 36px;
  height: 36px;
  min-width: 36px;
  min-height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-weight: 600;
  flex-shrink: 0;
}

.chat-message.user .message-avatar {
  background: #1F2937;
  color: #FFFFFF;
}

.chat-message.assistant .message-avatar {
  background: #F3F4F6;
  color: #374151;
}

.message-content {
  max-width: 70%;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.chat-message.user .message-content {
  align-items: flex-end;
}

.message-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.chat-message.user .message-header {
  flex-direction: row-reverse;
}

.sender-name {
  font-size: 12px;
  font-weight: 600;
  color: #374151;
}

.message-time {
  font-size: 11px;
  color: #9CA3AF;
}

.message-text {
  padding: 10px 14px;
  border-radius: 12px;
  font-size: 14px;
  line-height: 1.5;
}

.chat-message.user .message-text {
  background: #1F2937;
  color: #FFFFFF;
  border-bottom-right-radius: 4px;
}

.chat-message.assistant .message-text {
  background: #F3F4F6;
  color: #374151;
  border-bottom-left-radius: 4px;
}

.message-text :deep(.md-p) {
  margin: 0;
}

.message-text :deep(.md-p:last-child) {
  margin-bottom: 0;
}

/* FIXME Sequence List Numbering - Use CSS counters for continuous numbering of multiple ol units */
.message-text {
  counter-reset: list-counter;
}

.message-text :deep(.md-ol) {
  list-style: none;
  padding-left: 0;
  margin: 8px 0;
}

.message-text :deep(.md-oli) {
  counter-increment: list-counter;
  display: flex;
  gap: 8px;
  margin: 4px 0;
}

.message-text :deep(.md-oli)::before {
  content: counter(list-counter) ".";
  font-weight: 600;
  color: #374151;
  min-width: 20px;
  flex-shrink: 0;
}

/* Unordered List Styles */
.message-text :deep(.md-ul) {
  padding-left: 20px;
  margin: 8px 0;
}

.message-text :deep(.md-li) {
  margin: 4px 0;
}

/* Typing Indicator */
.typing-indicator {
  display: flex;
  gap: 4px;
  padding: 10px 14px;
  background: #F3F4F6;
  border-radius: 12px;
  border-bottom-left-radius: 4px;
}

.typing-indicator span {
  width: 8px;
  height: 8px;
  background: #9CA3AF;
  border-radius: 50%;
  animation: typing 1.4s infinite ease-in-out;
}

.typing-indicator span:nth-child(1) { animation-delay: 0s; }
.typing-indicator span:nth-child(2) { animation-delay: 0.2s; }
.typing-indicator span:nth-child(3) { animation-delay: 0.4s; }

@keyframes typing {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-8px); }
}

/* Chat Input */
.chat-input-area {
  padding: 16px 24px;
  border-top: 1px solid #E5E7EB;
  display: flex;
  gap: 12px;
  align-items: flex-end;
}

.chat-input {
  flex: 1;
  padding: 12px 16px;
  font-size: 14px;
  border: 1px solid #E5E7EB;
  border-radius: 8px;
  resize: none;
  font-family: inherit;
  line-height: 1.5;
  transition: border-color 0.2s ease;
}

.chat-input:focus {
  outline: none;
  border-color: #1F2937;
}

.chat-input:disabled {
  background: #F9FAFB;
  cursor: not-allowed;
}

.send-btn {
  width: 44px;
  height: 44px;
  background: #1F2937;
  color: #FFFFFF;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s ease;
}

.send-btn:hover:not(:disabled) {
  background: #374151;
}

.send-btn:disabled {
  background: #E5E7EB;
  color: #9CA3AF;
  cursor: not-allowed;
}

/* Survey Container */
.survey-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.survey-setup {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 24px;
  border-bottom: 1px solid #E5E7EB;
  overflow: hidden;
}

.setup-section {
  margin-bottom: 24px;
}

.setup-section:first-child {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}

.setup-section:last-child {
  margin-bottom: 0;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.setup-section .section-header .section-title {
  font-size: 13px;
  font-weight: 600;
  color: #374151;
}

.selection-count {
  font-size: 12px;
  color: #9CA3AF;
}

/* Agents Grid */
.agents-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
  flex: 1;
  overflow-y: auto;
  padding: 4px;
  align-content: start;
}

.agent-checkbox {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: #F9FAFB;
  border: 1px solid #E5E7EB;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.agent-checkbox:hover {
  border-color: #D1D5DB;
}

.agent-checkbox.checked {
  background: #F0FDF4;
  border-color: #10B981;
}

.agent-checkbox input {
  display: none;
}

.checkbox-avatar {
  width: 28px;
  height: 28px;
  min-width: 28px;
  min-height: 28px;
  background: #E5E7EB;
  color: #374151;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-weight: 600;
  flex-shrink: 0;
}

.agent-checkbox.checked .checkbox-avatar {
  background: #10B981;
  color: #FFFFFF;
}

.checkbox-info {
  flex: 1;
  min-width: 0;
}

.checkbox-name {
  display: block;
  font-size: 12px;
  font-weight: 600;
  color: #1F2937;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.checkbox-role {
  display: block;
  font-size: 10px;
  color: #9CA3AF;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.checkbox-indicator {
  width: 20px;
  height: 20px;
  border: 2px solid #E5E7EB;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.2s ease;
}

.agent-checkbox.checked .checkbox-indicator {
  background: #10B981;
  border-color: #10B981;
  color: #FFFFFF;
}

.checkbox-indicator svg {
  opacity: 0;
  transform: scale(0.5);
  transition: all 0.2s ease;
}

.agent-checkbox.checked .checkbox-indicator svg {
  opacity: 1;
  transform: scale(1);
}

.selection-actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.action-link {
  font-size: 12px;
  color: #6B7280;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
}

.action-link:hover {
  color: #1F2937;
  text-decoration: underline;
}

.action-divider {
  color: #E5E7EB;
}

/* Survey Input */
.survey-input {
  width: 100%;
  padding: 14px 16px;
  font-size: 14px;
  border: 1px solid #E5E7EB;
  border-radius: 8px;
  resize: none;
  font-family: inherit;
  line-height: 1.5;
  transition: border-color 0.2s ease;
}

.survey-input:focus {
  outline: none;
  border-color: #1F2937;
}

.survey-submit-btn {
  width: 100%;
  padding: 14px 24px;
  font-size: 14px;
  font-weight: 600;
  color: #FFFFFF;
  background: #1F2937;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-top: 20px;
}

.survey-submit-btn:hover:not(:disabled) {
  background: #374151;
}

.survey-submit-btn:disabled {
  background: #E5E7EB;
  color: #9CA3AF;
  cursor: not-allowed;
}

.loading-spinner {
  width: 18px;
  height: 18px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #FFFFFF;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Survey Results */
.survey-results {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}

.results-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.results-title {
  font-size: 14px;
  font-weight: 600;
  color: #1F2937;
}

.results-count {
  font-size: 12px;
  color: #9CA3AF;
}

.results-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.result-card {
  background: #F9FAFB;
  border: 1px solid #E5E7EB;
  border-radius: 12px;
  padding: 20px;
}

.result-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.result-avatar {
  width: 36px;
  height: 36px;
  min-width: 36px;
  min-height: 36px;
  background: #1F2937;
  color: #FFFFFF;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-weight: 600;
  flex-shrink: 0;
}

.result-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.result-name {
  font-size: 14px;
  font-weight: 600;
  color: #1F2937;
}

.result-role {
  font-size: 12px;
  color: #9CA3AF;
}

.result-question {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 12px 14px;
  background: #FFFFFF;
  border-radius: 8px;
  margin-bottom: 12px;
  font-size: 13px;
  color: #6B7280;
}

.result-question svg {
  flex-shrink: 0;
  margin-top: 2px;
}

.result-answer {
  font-size: 14px;
  line-height: 1.7;
  color: #374151;
}

/* Markdown Styles */
:deep(.md-p) {
  margin: 0 0 12px 0;
}

:deep(.md-h2) {
  font-size: 20px;
  font-weight: 700;
  color: #1F2937;
  margin: 24px 0 12px 0;
}

:deep(.md-h3) {
  font-size: 16px;
  font-weight: 600;
  color: #374151;
  margin: 20px 0 10px 0;
}

:deep(.md-h4) {
  font-size: 14px;
  font-weight: 600;
  color: #4B5563;
  margin: 16px 0 8px 0;
}

:deep(.md-h5) {
  font-size: 13px;
  font-weight: 600;
  color: #6B7280;
  margin: 12px 0 6px 0;
}

:deep(.md-ul), :deep(.md-ol) {
  margin: 12px 0;
  padding-left: 24px;
}

:deep(.md-li), :deep(.md-oli) {
  margin: 6px 0;
}

/* Chat/Survey Area Quote Styles */
.chat-messages :deep(.md-quote),
.result-answer :deep(.md-quote) {
  margin: 12px 0;
  padding: 12px 16px;
  background: #F9FAFB;
  border-left: 3px solid #1F2937;
  color: #4B5563;
}

:deep(.code-block) {
  margin: 12px 0;
  padding: 12px 16px;
  background: #1F2937;
  border-radius: 6px;
  overflow-x: auto;
}

:deep(.code-block code) {
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  color: #E5E7EB;
}

:deep(.inline-code) {
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  background: #F3F4F6;
  padding: 2px 6px;
  border-radius: 4px;
  color: #1F2937;
}

:deep(.md-hr) {
  border: none;
  border-top: 1px solid #E5E7EB;
  margin: 24px 0;
}

/* ============ Panel Discussion Styles ============ */
.panel-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-header {
  padding: 16px 20px;
  border-bottom: 1px solid #E5E7EB;
  background: #FAFAFA;
}

.participants-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.participants-label {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.participants-label .label-text {
  font-size: 13px;
  font-weight: 600;
  color: #374151;
}

.participant-count {
  font-size: 12px;
  color: #9CA3AF;
}

.participants-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}

.participant-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px 4px 4px;
  background: #FFFFFF;
  border: 1px solid #E5E7EB;
  border-radius: 20px;
  font-size: 12px;
}

.chip-avatar {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: linear-gradient(135deg, #6366F1, #8B5CF6);
  color: #FFFFFF;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: 600;
}

.chip-name {
  color: #374151;
  max-width: 80px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chip-remove {
  background: none;
  border: none;
  color: #9CA3AF;
  cursor: pointer;
  font-size: 14px;
  padding: 0 2px;
  line-height: 1;
}

.chip-remove:hover {
  color: #EF4444;
}

/* Mute toggle button */
.chip-mute {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 11px;
  padding: 0 2px;
  line-height: 1;
  opacity: 0.6;
  transition: opacity 0.2s;
}

.chip-mute:hover {
  opacity: 1;
}

/* Muted participant chip */
.participant-chip.muted {
  opacity: 0.5;
  border-style: dashed;
  background: #F9FAFB;
}

.participant-chip.muted .chip-avatar {
  background: #9CA3AF;
}

.more-chip {
  background: #F3F4F6;
  color: #6B7280;
  font-weight: 500;
}

/* Injection indicator and badges */
.injection-indicator {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  font-size: 11px;
  border-radius: 12px;
  font-weight: 500;
}

.injection-indicator.global {
  background: linear-gradient(135deg, #EEF2FF 0%, #E0E7FF 100%);
  color: #6366F1;
  border: 1px solid #C7D2FE;
}

.chip-injection-badge {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 2px 6px;
  font-size: 10px;
  background: #6366F1;
  color: #FFF;
  border-radius: 10px;
  font-weight: 600;
  border: none;
}

.chip-injection-badge.clickable,
.injection-indicator.clickable,
.agent-injection-badge {
  cursor: pointer;
  transition: all 0.15s;
}

.chip-injection-badge.clickable:hover,
.injection-indicator.clickable:hover,
.agent-injection-badge:hover {
  transform: scale(1.05);
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.4);
}

.agent-injection-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  font-size: 11px;
  background: linear-gradient(135deg, #6366F1 0%, #8B5CF6 100%);
  color: #FFF;
  border: none;
  border-radius: 16px;
  font-weight: 600;
  margin-left: auto;
  margin-right: 8px;
}

/* Removal Modal */
.removal-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.removal-modal {
  background: #FFF;
  border-radius: 16px;
  width: 400px;
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.25);
  animation: modalSlideIn 0.2s ease;
}

.removal-modal-header {
  padding: 16px 20px;
  border-bottom: 1px solid #E5E7EB;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.removal-modal-header h3 {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
}

.removal-modal-close {
  background: none;
  border: none;
  font-size: 22px;
  color: #9CA3AF;
  cursor: pointer;
  line-height: 1;
}

.removal-modal-body {
  padding: 16px 20px;
  overflow-y: auto;
  flex: 1;
}

.removal-empty {
  text-align: center;
  padding: 24px;
  color: #9CA3AF;
}

.removal-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.removal-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px;
  background: #F9FAFB;
  border-radius: 8px;
  border: 1px solid #E5E7EB;
}

.removal-text {
  flex: 1;
  font-size: 12px;
  color: #374151;
  font-style: italic;
  margin: 0;
  line-height: 1.5;
}

.removal-btn {
  background: none;
  border: none;
  font-size: 14px;
  cursor: pointer;
  padding: 4px;
  opacity: 0.6;
  transition: opacity 0.15s;
}

.removal-btn:hover {
  opacity: 1;
}

.removal-modal-footer {
  padding: 12px 20px;
  border-top: 1px solid #E5E7EB;
  display: flex;
  justify-content: space-between;
  gap: 12px;
}

.removal-clear-btn {
  padding: 8px 16px;
  font-size: 12px;
  font-weight: 600;
  background: #FEE2E2;
  border: 1px solid #FCA5A5;
  color: #DC2626;
  border-radius: 6px;
  cursor: pointer;
}

.removal-clear-btn:hover {
  background: #FECACA;
}

.removal-done-btn {
  padding: 8px 16px;
  font-size: 12px;
  font-weight: 600;
  background: #6366F1;
  border: none;
  color: #FFF;
  border-radius: 6px;
  cursor: pointer;
  margin-left: auto;
}

.removal-done-btn:hover {
  background: #4F46E5;
}

.add-participants-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: #1F2937;
  color: #FFFFFF;
  border: none;
  border-radius: 20px;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.2s;
}

.add-participants-btn:hover {
  background: #374151;
}

/* Panel Thread */
.panel-thread {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.empty-thread {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #9CA3AF;
  text-align: center;
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 12px;
}

.empty-thread p {
  max-width: 280px;
  line-height: 1.5;
}

.exchange-block {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.exchange-question {
  display: flex;
  align-items: flex-end;
  gap: 8px;
}

.question-bubble {
  background: #1F2937;
  color: #FFFFFF;
  padding: 12px 16px;
  border-radius: 16px 16px 4px 16px;
  max-width: 80%;
}

.question-label {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.6);
  display: block;
  margin-bottom: 4px;
}

.question-bubble p {
  margin: 0;
  font-size: 14px;
  line-height: 1.5;
}

.exchange-time {
  font-size: 11px;
  color: #9CA3AF;
}

.exchange-responses {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-left: 20px;
}

.response-card {
  background: #F9FAFB;
  border: 1px solid #E5E7EB;
  border-radius: 12px;
  padding: 14px;
}

.resp-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.resp-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: linear-gradient(135deg, #10B981, #059669);
  color: #FFFFFF;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
}

.resp-info {
  display: flex;
  flex-direction: column;
}

.resp-name {
  font-size: 13px;
  font-weight: 600;
  color: #1F2937;
}

.resp-role {
  font-size: 11px;
  color: #9CA3AF;
}

.resp-content {
  font-size: 14px;
  line-height: 1.7;
  color: #374151;
}

/* Reply button on response cards */
.resp-reply-btn {
  margin-left: auto;
  background: none;
  border: 1px solid #E5E7EB;
  border-radius: 6px;
  padding: 4px 10px;
  font-size: 11px;
  color: #6B7280;
  cursor: pointer;
  transition: all 0.2s;
  opacity: 0;
}

.response-card:hover .resp-reply-btn {
  opacity: 1;
}

.resp-reply-btn:hover {
  background: #F3F4F6;
  border-color: #9CA3AF;
  color: #374151;
}

/* Panel Input Area */
.panel-input-area {
  padding: 16px 20px;
  border-top: 1px solid #E5E7EB;
  background: #FFFFFF;
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: flex-end;
}

/* Quote indicator */
.quote-indicator {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: linear-gradient(135deg, #6366F1 0%, #8B5CF6 100%);
  border-radius: 8px;
  margin-bottom: 4px;
}

.quote-text {
  font-size: 12px;
  color: #FFFFFF;
  font-weight: 500;
}

.quote-clear {
  background: rgba(255, 255, 255, 0.2);
  border: none;
  border-radius: 4px;
  color: #FFFFFF;
  width: 20px;
  height: 20px;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
}

.quote-clear:hover {
  background: rgba(255, 255, 255, 0.3);
}

/* Selection Popup for Knowledge Pad */
.selection-popup {
  position: fixed;
  transform: translate(-50%, -100%);
  z-index: 10000;
  animation: fadeIn 0.15s ease;
}

.add-knowledge-btn {
  background: linear-gradient(135deg, #6366F1 0%, #8B5CF6 100%);
  border: none;
  color: #FFF;
  padding: 8px 16px;
  font-size: 12px;
  font-weight: 600;
  border-radius: 8px;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
  white-space: nowrap;
  transition: all 0.2s;
}

.add-knowledge-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(99, 102, 241, 0.5);
}

@keyframes fadeIn {
  from { opacity: 0; transform: translate(-50%, -90%); }
  to { opacity: 1; transform: translate(-50%, -100%); }
}

/* Tag Modal */
.tag-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.tag-modal {
  background: #FFF;
  border-radius: 16px;
  width: 440px;
  max-width: 90vw;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.25);
  animation: modalSlideIn 0.2s ease;
}

@keyframes modalSlideIn {
  from { opacity: 0; transform: translateY(-20px); }
  to { opacity: 1; transform: translateY(0); }
}

.tag-modal-header {
  padding: 20px 24px;
  border-bottom: 1px solid #E5E7EB;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.tag-modal-header h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
}

.tag-modal-close {
  background: none;
  border: none;
  font-size: 24px;
  color: #9CA3AF;
  cursor: pointer;
  line-height: 1;
}

.tag-modal-body {
  padding: 24px;
}

.tag-preview {
  background: #F9FAFB;
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 20px;
}

.preview-text {
  font-size: 13px;
  font-style: italic;
  color: #374151;
  margin: 0 0 8px 0;
  line-height: 1.5;
}

.preview-source {
  font-size: 11px;
  color: #9CA3AF;
  margin: 0;
}

.tag-section {
  margin-bottom: 16px;
}

.tag-label {
  display: block;
  font-size: 12px;
  font-weight: 600;
  color: #374151;
  margin-bottom: 8px;
}

.predefined-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.tag-chip {
  padding: 6px 14px;
  font-size: 12px;
  border: 1px solid #E5E7EB;
  border-radius: 20px;
  background: #FFF;
  color: #6B7280;
  cursor: pointer;
  transition: all 0.15s;
}

.tag-chip:hover {
  border-color: #6366F1;
  color: #6366F1;
}

.tag-chip.selected {
  background: #6366F1;
  border-color: #6366F1;
  color: #FFF;
}

.custom-tag-section {
  margin-bottom: 16px;
}

.custom-tag-input {
  display: flex;
  gap: 8px;
}

.custom-tag-input input {
  flex: 1;
  padding: 10px 14px;
  font-size: 13px;
  border: 1px solid #E5E7EB;
  border-radius: 8px;
  outline: none;
}

.custom-tag-input input:focus {
  border-color: #6366F1;
}

.add-tag-btn {
  width: 40px;
  background: #6366F1;
  border: none;
  border-radius: 8px;
  color: #FFF;
  font-size: 18px;
  cursor: pointer;
}

.add-tag-btn:hover {
  background: #4F46E5;
}

.selected-tags-preview {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
  padding: 12px;
  background: #EEF2FF;
  border-radius: 8px;
}

.tags-label {
  font-size: 11px;
  color: #6366F1;
  font-weight: 600;
}

.selected-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  background: #6366F1;
  color: #FFF;
  font-size: 11px;
  border-radius: 12px;
}

.selected-tag button {
  background: none;
  border: none;
  color: rgba(255,255,255,0.7);
  font-size: 12px;
  cursor: pointer;
  padding: 0;
  line-height: 1;
}

.selected-tag button:hover {
  color: #FFF;
}

.tag-modal-footer {
  padding: 16px 24px;
  border-top: 1px solid #E5E7EB;
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.tag-btn {
  padding: 10px 20px;
  font-size: 13px;
  font-weight: 600;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.tag-btn.secondary {
  background: #FFF;
  border: 1px solid #E5E7EB;
  color: #374151;
}

.tag-btn.secondary:hover {
  background: #F3F4F6;
}

.tag-btn.primary {
  background: linear-gradient(135deg, #6366F1 0%, #8B5CF6 100%);
  border: none;
  color: #FFF;
}

.tag-btn.primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
}

.panel-input {
  flex: 1;
  padding: 12px 16px;
  font-size: 14px;
  border: 1px solid #E5E7EB;
  border-radius: 12px;
  resize: none;
  font-family: inherit;
}

.panel-input:focus {
  outline: none;
  border-color: #1F2937;
}

.panel-send-btn {
  padding: 12px 24px;
  font-size: 14px;
  font-weight: 600;
  color: #FFFFFF;
  background: #1F2937;
  border: none;
  border-radius: 12px;
  cursor: pointer;
  transition: background 0.2s;
  display: flex;
  align-items: center;
  gap: 8px;
}

.panel-send-btn:hover:not(:disabled) {
  background: #374151;
}

.panel-send-btn:disabled {
  background: #E5E7EB;
  color: #9CA3AF;
  cursor: not-allowed;
}

/* Agora Container */

.agora-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* --- Mini Logs Panel (Step 5 Style) --- */
.system-logs-mini {
  background: #0f172a;
  border-top: 1px solid #1e293b;
  height: 180px;
  display: flex;
  flex-direction: column;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  position: relative;
  z-index: 100;
}

.logs-header {
  padding: 8px 16px;
  background: #1e293b;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.logs-title {
  color: #94a3b8;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.1em;
}

.clear-logs-btn {
  background: transparent;
  border: 1px solid #334155;
  color: #64748b;
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
}

.clear-logs-btn:hover {
  background: #334155;
  color: #f1f5f9;
}

.logs-content {
  flex: 1;
  overflow-y: auto;
  padding: 10px 16px;
  background: #020617;
}

.log-entry {
  font-size: 12px;
  line-height: 1.5;
  margin-bottom: 4px;
  display: flex;
  gap: 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  padding-bottom: 2px;
}

.log-entry-time {
  color: #64748b;
  white-space: nowrap;
  flex-shrink: 0;
}

.log-entry-msg {
  color: #cbd5e1;
  word-break: break-all;
}

.log-warning .log-entry-msg {
  color: #fbbf24;
}

.log-error .log-entry-msg {
  color: #f87171;
}

.logs-empty {
  color: #475569;
  font-size: 12px;
  text-align: center;
  margin-top: 20px;
  font-style: italic;
}

/* Custom Scrollbar for Logs */
.logs-content::-webkit-scrollbar {
  width: 6px;
}
.logs-content::-webkit-scrollbar-track {
  background: #020617;
}
.logs-content::-webkit-scrollbar-thumb {
  background: #1e293b;
  border-radius: 3px;
}
.logs-content::-webkit-scrollbar-thumb:hover {
  background: #334155;
}

.agora-pill.active {
  background: linear-gradient(135deg, #ff5722, #6366f1) !important;
  color: white !important;
}
</style>
