<script lang="ts">
	import { user } from '$lib/stores/auth';
</script>

<div class="min-h-screen bg-tw-darkblue flex flex-col items-center px-4 pt-12">
	<h1
		class="text-5xl sm:text-6xl font-extrabold mb-10 pb-2
               leading-normal
               bg-linear-to-r from-tw-purple to-tw-pink
               bg-clip-text text-transparent
               drop-shadow-[0_0_12px_rgba(0,245,255,0.5)]"
	>
		ImgCat
	</h1>

	<div
		class="w-full max-w-2xl bg-white/5 backdrop-blur
             border border-tw-purple/30 rounded-2xl p-8
             flex flex-col gap-5"
	>
		<p class="text-tw-neon text-xl font-semibold">
			Welcome back, {$user?.username}
		</p>
		<p class="text-white/60 text-sm">This is where the magic happens.</p>
	</div>
</div>
