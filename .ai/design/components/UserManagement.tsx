import { useState } from 'react';
import { Users, UserPlus, Edit2, Trash2, AlertTriangle, CheckCircle2, RefreshCw, Eye, EyeOff } from 'lucide-react';

interface User {
  id: string;
  name: string;
  password: string;
  role: 'administrator' | 'operator' | 'user';
}

export default function UserManagement() {
  const [users, setUsers] = useState<User[]>([
    { id: '1', name: 'admin', password: 'admin', role: 'administrator' },
    { id: '2', name: 'operator1', password: 'operator123', role: 'operator' },
    { id: '3', name: 'viewer', password: 'user123', role: 'user' },
  ]);

  const [showAddModal, setShowAddModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [selectedUser, setSelectedUser] = useState<User | null>(null);
  const [actionStatus, setActionStatus] = useState<'idle' | 'processing'>('idle');

  // Form states
  const [formName, setFormName] = useState('');
  const [formPassword, setFormPassword] = useState('');
  const [formRole, setFormRole] = useState<'administrator' | 'operator' | 'user'>('user');
  const [showPassword, setShowPassword] = useState(false);

  const resetForm = () => {
    setFormName('');
    setFormPassword('');
    setFormRole('user');
    setShowPassword(false);
  };

  const handleAddUser = () => {
    setActionStatus('processing');
    setTimeout(() => {
      const newUser: User = {
        id: Date.now().toString(),
        name: formName,
        password: formPassword,
        role: formRole,
      };
      setUsers([...users, newUser]);
      setActionStatus('idle');
      setShowAddModal(false);
      resetForm();
    }, 1000);
  };

  const handleEditUser = () => {
    if (!selectedUser) return;
    setActionStatus('processing');
    setTimeout(() => {
      setUsers(users.map(u => u.id === selectedUser.id ? {
        ...u,
        name: formName,
        password: formPassword,
        role: formRole,
      } : u));
      setActionStatus('idle');
      setShowEditModal(false);
      setSelectedUser(null);
      resetForm();
    }, 1000);
  };

  const handleDeleteUser = () => {
    if (!selectedUser) return;
    setActionStatus('processing');
    setTimeout(() => {
      setUsers(users.filter(u => u.id !== selectedUser.id));
      setActionStatus('idle');
      setShowDeleteModal(false);
      setSelectedUser(null);
    }, 1000);
  };

  const openEditModal = (user: User) => {
    setSelectedUser(user);
    setFormName(user.name);
    setFormPassword(user.password);
    setFormRole(user.role);
    setShowEditModal(true);
  };

  const openDeleteModal = (user: User) => {
    setSelectedUser(user);
    setShowDeleteModal(true);
  };

  const getRoleColor = (role: string) => {
    switch (role) {
      case 'administrator':
        return 'text-[#dc2626] bg-[rgba(220,38,38,0.1)]';
      case 'operator':
        return 'text-[#ff9f0a] bg-[rgba(255,159,10,0.1)]';
      case 'user':
        return 'text-[#007AFF] bg-[rgba(0,122,255,0.1)]';
      default:
        return 'text-[#a1a1a6] bg-[#2c2c2e]';
    }
  };

  const getRoleLabel = (role: string) => {
    return role.charAt(0).toUpperCase() + role.slice(1);
  };

  return (
    <div className="absolute inset-0 lg:inset-[0_0_0_356.84px] overflow-auto bg-[#0d0d0d]" data-name="Container">
      <div className="max-w-[1100px] p-[16px] md:p-[32px] lg:p-[48px] pb-[80px] md:pb-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px] flex flex-col sm:flex-row items-start sm:items-center justify-between gap-[16px]">
          <div>
            <h1 className="text-white text-[22px] md:text-[28px] mb-[8px]">User Management</h1>
            <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">
              Manage user accounts, passwords, and access permissions
            </p>
          </div>
          <button
            onClick={() => setShowAddModal(true)}
            className="w-full sm:w-auto h-[44px] px-[24px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors flex items-center justify-center gap-[8px]"
          >
            <UserPlus className="size-5" />
            Add User
          </button>
        </div>

        {/* Users List */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] overflow-hidden">
          {/* Table Header - Hidden on mobile */}
          <div className="hidden md:grid grid-cols-[2fr_2fr_1.5fr_120px] gap-[24px] px-[32px] py-[16px] border-b border-[#3a3a3c] bg-[#0d0d0d]">
            <div className="text-[#a1a1a6] text-[14px] font-semibold">Username</div>
            <div className="text-[#a1a1a6] text-[14px] font-semibold">Password</div>
            <div className="text-[#a1a1a6] text-[14px] font-semibold">Role</div>
            <div className="text-[#a1a1a6] text-[14px] font-semibold">Actions</div>
          </div>

          {/* Table Body - Desktop */}
          <div className="hidden md:block">
            {users.map((user, index) => (
              <div 
                key={user.id}
                className={`grid grid-cols-[2fr_2fr_1.5fr_120px] gap-[24px] px-[32px] py-[20px] hover:bg-[#2c2c2e] transition-colors ${
                  index !== users.length - 1 ? 'border-b border-[#3a3a3c]' : ''
                }`}
              >
                <div className="flex items-center">
                  <div className="size-[40px] rounded-full bg-[#dc2626] flex items-center justify-center mr-[12px]">
                    <span className="text-white font-semibold text-[15px]">
                      {user.name.charAt(0).toUpperCase()}
                    </span>
                  </div>
                  <span className="text-white text-[15px]">{user.name}</span>
                </div>
                <div className="flex items-center text-[#a1a1a6] text-[15px]">
                  {'•'.repeat(user.password.length)}
                </div>
                <div className="flex items-center">
                  <span className={`px-[12px] py-[6px] rounded-[6px] text-[14px] font-semibold ${getRoleColor(user.role)}`}>
                    {getRoleLabel(user.role)}
                  </span>
                </div>
                <div className="flex items-center gap-[8px]">
                  <button
                    onClick={() => openEditModal(user)}
                    className="size-[36px] rounded-[8px] bg-transparent border border-[#3a3a3c] flex items-center justify-center hover:bg-[#2c2c2e] transition-colors"
                  >
                    <Edit2 className="size-4" color="#a1a1a6" />
                  </button>
                  <button
                    onClick={() => openDeleteModal(user)}
                    disabled={user.role === 'administrator' && users.filter(u => u.role === 'administrator').length === 1}
                    className="size-[36px] rounded-[8px] bg-transparent border border-[#3a3a3c] flex items-center justify-center hover:bg-[rgba(220,38,38,0.1)] hover:border-[#dc2626] transition-colors disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent disabled:hover:border-[#3a3a3c]"
                  >
                    <Trash2 className="size-4" color="#dc2626" />
                  </button>
                </div>
              </div>
            ))}
          </div>

          {/* Mobile Card View */}
          <div className="md:hidden">
            {users.map((user, index) => (
              <div 
                key={user.id}
                className={`p-[16px] ${
                  index !== users.length - 1 ? 'border-b border-[#3a3a3c]' : ''
                }`}
              >
                {/* User Header */}
                <div className="flex items-center justify-between mb-[12px]">
                  <div className="flex items-center gap-[12px] flex-1 min-w-0">
                    <div className="size-[40px] rounded-full bg-[#dc2626] flex items-center justify-center flex-shrink-0">
                      <span className="text-white font-semibold text-[15px]">
                        {user.name.charAt(0).toUpperCase()}
                      </span>
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="text-white text-[15px] mb-[4px]">{user.name}</div>
                      <div className="text-[#a1a1a6] text-[13px]">
                        {'•'.repeat(user.password.length)}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-[8px] flex-shrink-0 ml-[12px]">
                    <button
                      onClick={() => openEditModal(user)}
                      className="size-[36px] rounded-[8px] bg-transparent border border-[#3a3a3c] flex items-center justify-center hover:bg-[#2c2c2e] transition-colors"
                    >
                      <Edit2 className="size-4" color="#a1a1a6" />
                    </button>
                    <button
                      onClick={() => openDeleteModal(user)}
                      disabled={user.role === 'administrator' && users.filter(u => u.role === 'administrator').length === 1}
                      className="size-[36px] rounded-[8px] bg-transparent border border-[#3a3a3c] flex items-center justify-center hover:bg-[rgba(220,38,38,0.1)] hover:border-[#dc2626] transition-colors disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent disabled:hover:border-[#3a3a3c]"
                    >
                      <Trash2 className="size-4" color="#dc2626" />
                    </button>
                  </div>
                </div>
                
                {/* Role Badge */}
                <div className="flex items-center">
                  <span className="text-[#6b6b6f] text-[13px] mr-[8px]">Role:</span>
                  <span className={`px-[10px] py-[4px] rounded-[6px] text-[13px] font-semibold ${getRoleColor(user.role)}`}>
                    {getRoleLabel(user.role)}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Info */}
        <div className="mt-[24px] p-[16px] bg-[rgba(0,122,255,0.05)] border border-[rgba(0,122,255,0.2)] rounded-[8px]">
          <div className="flex gap-[12px]">
            <Users className="size-5 text-[#007AFF] flex-shrink-0 mt-[2px]" />
            <div>
              <p className="text-[#007AFF] text-[14px] mb-[4px]">User Roles</p>
              <p className="text-[#a1a1a6] text-[13px]">
                <strong className="text-white">Administrator:</strong> Full access to all settings and features. 
                <strong className="text-white ml-[12px]">Operator:</strong> Can view and modify device settings. 
                <strong className="text-white ml-[12px]">User:</strong> View-only access.
              </p>
            </div>
          </div>
        </div>

        {/* Add User Modal */}
        {showAddModal && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] max-w-[520px] w-full mx-[24px]">
              <div className="flex items-center gap-[16px] mb-[24px]">
                <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
                  <UserPlus className="size-6" color="#dc2626" />
                </div>
                <h3 className="text-white text-[20px]">Add New User</h3>
              </div>

              <div className="space-y-[20px] mb-[24px]">
                {/* Username */}
                <div>
                  <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                    Username
                  </label>
                  <input
                    type="text"
                    value={formName}
                    onChange={(e) => setFormName(e.target.value)}
                    className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors"
                    placeholder="Enter username"
                  />
                </div>

                {/* Password */}
                <div>
                  <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                    Password
                  </label>
                  <div className="relative">
                    <input
                      type={showPassword ? 'text' : 'password'}
                      value={formPassword}
                      onChange={(e) => setFormPassword(e.target.value)}
                      className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] pr-[48px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors"
                      placeholder="Enter password"
                    />
                    <button
                      onClick={() => setShowPassword(!showPassword)}
                      className="absolute right-[12px] top-1/2 translate-y-[-50%] p-[4px]"
                    >
                      {showPassword ? (
                        <EyeOff className="size-5" color="#a1a1a6" />
                      ) : (
                        <Eye className="size-5" color="#a1a1a6" />
                      )}
                    </button>
                  </div>
                </div>

                {/* Role */}
                <div>
                  <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                    Role
                  </label>
                  <div className="relative">
                    <select
                      value={formRole}
                      onChange={(e) => setFormRole(e.target.value as any)}
                      className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] pr-[40px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors appearance-none cursor-pointer"
                    >
                      <option value="user">User</option>
                      <option value="operator">Operator</option>
                      <option value="administrator">Administrator</option>
                    </select>
                    <div className="absolute right-[16px] top-1/2 translate-y-[-50%] pointer-events-none">
                      <svg className="size-4" fill="none" viewBox="0 0 24 24" stroke="#a1a1a6" strokeWidth="2">
                        <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                      </svg>
                    </div>
                  </div>
                </div>
              </div>

              <div className="flex gap-[16px]">
                <button
                  onClick={handleAddUser}
                  disabled={actionStatus === 'processing' || !formName || !formPassword}
                  className="flex-1 h-[44px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-[8px]"
                >
                  {actionStatus === 'processing' ? (
                    <>
                      <RefreshCw className="size-4 animate-spin" />
                      Adding...
                    </>
                  ) : (
                    'Add User'
                  )}
                </button>
                <button
                  onClick={() => {
                    setShowAddModal(false);
                    resetForm();
                  }}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Edit User Modal */}
        {showEditModal && selectedUser && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] max-w-[520px] w-full mx-[24px]">
              <div className="flex items-center gap-[16px] mb-[24px]">
                <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
                  <Edit2 className="size-6" color="#dc2626" />
                </div>
                <h3 className="text-white text-[20px]">Edit User</h3>
              </div>

              <div className="space-y-[20px] mb-[24px]">
                {/* Username */}
                <div>
                  <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                    Username
                  </label>
                  <input
                    type="text"
                    value={formName}
                    onChange={(e) => setFormName(e.target.value)}
                    className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors"
                    placeholder="Enter username"
                  />
                </div>

                {/* Password */}
                <div>
                  <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                    Password
                  </label>
                  <div className="relative">
                    <input
                      type={showPassword ? 'text' : 'password'}
                      value={formPassword}
                      onChange={(e) => setFormPassword(e.target.value)}
                      className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] pr-[48px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors"
                      placeholder="Enter password"
                    />
                    <button
                      onClick={() => setShowPassword(!showPassword)}
                      className="absolute right-[12px] top-1/2 translate-y-[-50%] p-[4px]"
                    >
                      {showPassword ? (
                        <EyeOff className="size-5" color="#a1a1a6" />
                      ) : (
                        <Eye className="size-5" color="#a1a1a6" />
                      )}
                    </button>
                  </div>
                </div>

                {/* Role */}
                <div>
                  <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                    Role
                  </label>
                  <div className="relative">
                    <select
                      value={formRole}
                      onChange={(e) => setFormRole(e.target.value as any)}
                      className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] pr-[40px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors appearance-none cursor-pointer"
                    >
                      <option value="user">User</option>
                      <option value="operator">Operator</option>
                      <option value="administrator">Administrator</option>
                    </select>
                    <div className="absolute right-[16px] top-1/2 translate-y-[-50%] pointer-events-none">
                      <svg className="size-4" fill="none" viewBox="0 0 24 24" stroke="#a1a1a6" strokeWidth="2">
                        <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                      </svg>
                    </div>
                  </div>
                </div>
              </div>

              <div className="flex gap-[16px]">
                <button
                  onClick={handleEditUser}
                  disabled={actionStatus === 'processing' || !formName || !formPassword}
                  className="flex-1 h-[44px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-[8px]"
                >
                  {actionStatus === 'processing' ? (
                    <>
                      <RefreshCw className="size-4 animate-spin" />
                      Saving...
                    </>
                  ) : (
                    'Save Changes'
                  )}
                </button>
                <button
                  onClick={() => {
                    setShowEditModal(false);
                    setSelectedUser(null);
                    resetForm();
                  }}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Delete Confirmation Modal */}
        {showDeleteModal && selectedUser && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] max-w-[480px] w-full mx-[24px]">
              <div className="flex items-center gap-[16px] mb-[24px]">
                <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
                  <AlertTriangle className="size-6" color="#dc2626" />
                </div>
                <h3 className="text-white text-[20px]">Delete User</h3>
              </div>

              <p className="text-[#a1a1a6] text-[15px] mb-[24px]">
                Are you sure you want to delete user <strong className="text-white">"{selectedUser.name}"</strong>? This action cannot be undone.
              </p>

              <div className="flex gap-[16px]">
                <button
                  onClick={handleDeleteUser}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-[8px]"
                >
                  {actionStatus === 'processing' ? (
                    <>
                      <RefreshCw className="size-4 animate-spin" />
                      Deleting...
                    </>
                  ) : (
                    'Delete User'
                  )}
                </button>
                <button
                  onClick={() => {
                    setShowDeleteModal(false);
                    setSelectedUser(null);
                  }}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}