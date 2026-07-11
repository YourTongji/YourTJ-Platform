import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as React from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { ProfilePrivacy } from "@/lib/api/types";

const profileVisibilityOptions = [
  { value: "public", label: "所有人" },
  { value: "campus", label: "校园成员" },
  { value: "only_me", label: "仅自己" },
] as const;

const listVisibilityOptions = [
  { value: "public", label: "所有人" },
  { value: "campus", label: "校园成员" },
  { value: "followers", label: "关注者" },
  { value: "only_me", label: "仅自己" },
] as const;

const dmPolicyOptions = [
  { value: "everyone", label: "所有校园成员" },
  { value: "following", label: "仅我关注的人" },
  { value: "nobody", label: "不接受新私信" },
] as const;

function PrivacySelect({
  id,
  label,
  value,
  options,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  options: readonly { value: string; label: string }[];
  onChange: (value: string) => void;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <Select value={value} onValueChange={onChange}>
        <SelectTrigger id={id}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

export function ProfilePrivacySettings() {
  const { account, refreshMe } = useAuth();
  const queryClient = useQueryClient();
  const profile = useQuery({ queryKey: ["my-profile"], queryFn: api.myProfile });
  const privacy = useQuery({ queryKey: ["my-privacy"], queryFn: api.myPrivacy });
  const [displayName, setDisplayName] = React.useState("");
  const [handle, setHandle] = React.useState(account?.handle ?? "");
  const [bio, setBio] = React.useState("");
  const [website, setWebsite] = React.useState("");
  const [privacyForm, setPrivacyForm] = React.useState<ProfilePrivacy | null>(null);

  React.useEffect(() => {
    if (!profile.data) return;
    setDisplayName(profile.data.displayName ?? "");
    setBio(profile.data.bio ?? "");
    setWebsite(profile.data.website ?? "");
  }, [profile.data]);

  React.useEffect(() => setHandle(account?.handle ?? ""), [account]);

  React.useEffect(() => {
    if (privacy.data) setPrivacyForm(privacy.data);
  }, [privacy.data]);

  const saveProfile = useMutation({
    mutationFn: async () => {
      const nextHandle = handle.trim();
      const result = await api.updateMyProfile({
        displayName: displayName.trim() || null,
        bio: bio.trim() || null,
        website: website.trim() || null,
      });
      if (nextHandle && nextHandle !== account?.handle) {
        await api.updateMe({ handle: nextHandle });
        await refreshMe();
      }
      return result;
    },
    onSuccess: async () => {
      toast.success("公开资料已保存");
      await queryClient.invalidateQueries({ queryKey: ["my-profile"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "保存失败"),
  });

  const savePrivacy = useMutation({
    mutationFn: () => {
      if (!privacyForm) throw new Error("隐私设置尚未加载");
      return api.updateMyPrivacy(privacyForm);
    },
    onSuccess: async () => {
      toast.success("隐私设置已保存");
      await queryClient.invalidateQueries({ queryKey: ["my-privacy"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "保存失败"),
  });

  const updatePrivacy = <Key extends keyof ProfilePrivacy>(
    key: Key,
    value: ProfilePrivacy[Key],
  ) => setPrivacyForm((current) => current ? { ...current, [key]: value } : current);

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>公开资料</CardTitle>
          <CardDescription>
            邮箱永不公开；头像和封面只接受本人已通过审核的 OSS 图片，不能填写第三方 URL。
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="profile-handle">Handle</Label>
            <Input
              id="profile-handle"
              minLength={3}
              maxLength={30}
              value={handle}
              onChange={(event) => setHandle(event.target.value)}
              autoCapitalize="none"
              autoCorrect="off"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="profile-display-name">显示名称</Label>
            <Input
              id="profile-display-name"
              maxLength={50}
              value={displayName}
              onChange={(event) => setDisplayName(event.target.value)}
              placeholder="可选；未填写时显示 handle"
            />
          </div>
          <div className="space-y-2">
            <div className="flex items-center justify-between gap-3">
              <Label htmlFor="profile-bio">简介</Label>
              <span className="text-xs text-muted-foreground">{bio.length}/500</span>
            </div>
            <Textarea
              id="profile-bio"
              maxLength={500}
              value={bio}
              onChange={(event) => setBio(event.target.value)}
              placeholder="介绍你的方向、兴趣或正在做的事"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="profile-website">个人网站</Label>
            <Input
              id="profile-website"
              type="url"
              maxLength={2048}
              value={website}
              onChange={(event) => setWebsite(event.target.value)}
              placeholder="https://example.com"
            />
            <p className="text-xs text-muted-foreground">只允许 HTTPS 链接。</p>
          </div>
          <Button
            type="button"
            onClick={() => saveProfile.mutate()}
            disabled={profile.isLoading || profile.isError || saveProfile.isPending}
          >
            保存公开资料
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>资料与社交隐私</CardTitle>
          <CardDescription>
            公共板块中的帖子仍按板块规则展示，不会因隐藏个人资料而被改成私密内容。
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {privacyForm ? (
            <>
              <div className="grid gap-4 sm:grid-cols-2">
                <PrivacySelect
                  id="profile-visibility"
                  label="谁能查看个人资料"
                  value={privacyForm.profileVisibility}
                  options={profileVisibilityOptions}
                  onChange={(value) => updatePrivacy("profileVisibility", value as ProfilePrivacy["profileVisibility"])}
                />
                <PrivacySelect
                  id="dm-policy"
                  label="谁能发起新私信"
                  value={privacyForm.dmPolicy}
                  options={dmPolicyOptions}
                  onChange={(value) => updatePrivacy("dmPolicy", value as ProfilePrivacy["dmPolicy"])}
                />
                <PrivacySelect
                  id="followers-visibility"
                  label="谁能查看关注者列表"
                  value={privacyForm.followersVisibility}
                  options={listVisibilityOptions}
                  onChange={(value) => updatePrivacy("followersVisibility", value as ProfilePrivacy["followersVisibility"])}
                />
                <PrivacySelect
                  id="following-visibility"
                  label="谁能查看关注列表"
                  value={privacyForm.followingVisibility}
                  options={listVisibilityOptions}
                  onChange={(value) => updatePrivacy("followingVisibility", value as ProfilePrivacy["followingVisibility"])}
                />
              </div>
              <div className="flex items-center justify-between gap-4 rounded-lg border p-3">
                <div>
                  <Label htmlFor="profile-discoverable">允许被发现</Label>
                  <p className="mt-1 text-sm text-muted-foreground">
                    允许出现在第三方关注列表和未来的账号搜索中；精确 handle 直达仍受资料可见性控制。
                  </p>
                </div>
                <Switch
                  id="profile-discoverable"
                  checked={privacyForm.discoverable}
                  onCheckedChange={(value) => updatePrivacy("discoverable", value)}
                />
              </div>
              <Button
                type="button"
                variant="outline"
                onClick={() => savePrivacy.mutate()}
                disabled={savePrivacy.isPending}
              >
                保存隐私设置
              </Button>
            </>
          ) : (
            <p className="text-sm text-muted-foreground">
              {privacy.isError ? "隐私设置加载失败，请稍后重试。" : "正在加载隐私设置…"}
            </p>
          )}
        </CardContent>
      </Card>
    </>
  );
}
